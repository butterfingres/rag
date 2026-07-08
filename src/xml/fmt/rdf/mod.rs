use {
    crate::xml::{
        self, Entry, ParserError, PartialEntry, PartialFeed, Replaceable, TryFromRootError, ns,
        parser::{Content, ParseTagInto, TagParser},
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, QName, ResolveResult},
        reader::NsReader,
    },
};

pub const RDF: ResolveResult<'static> =
    ResolveResult::Bound(Namespace(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#"));
pub const RSS: ResolveResult<'static> =
    ResolveResult::Bound(Namespace(b"http://purl.org/rss/1.0/"));

struct RdfItemHandler;
impl<'alloc, 'src, F, T, A> ParseTagInto<'alloc, 'src, A, F> for RdfItemHandler
where
    F: FnMut(Entry<'alloc, 'src, A>) -> T,
    T: Into<Result<(), ParserError>>,
    A: Allocator + 'alloc,
{
    fn parse_tag_into(
        cb: &mut F,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut entry = PartialEntry::new_in(alloc);
        loop {
            match reader.read_resolved_event()? {
                (RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"title" => {
                    entry.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"description" => {
                    entry.content.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"link" => {
                    entry.link.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }

                (
                    ResolveResult::Bound(Namespace(ns)),
                    event @ (Event::Start(_) | Event::Empty(_)),
                ) if let Some(handler) = ns::item_handler(ns) => {
                    handler.handle_start(reader, event, &mut entry, version, alloc)?;
                }
                (_, Event::Start(tag)) => {
                    reader.read_to_end(tag.name())?;
                }

                (_, Event::End(tag)) if tag.name() == name => {
                    cb(entry.into()).into()?;
                    return Ok(());
                }
                (_, Event::Eof) => return Err(ParserError::UNCLOSED_TAG),

                _ => {}
            }
        }
    }
}

struct RdfChannel;
impl<'alloc, 'src, A> ParseTagInto<'alloc, 'src, A, PartialFeed<'alloc, 'src, A>> for RdfChannel
where
    A: Allocator,
{
    fn parse_tag_into(
        feed: &mut PartialFeed<'alloc, 'src, A>,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        loop {
            match reader.read_resolved_event()? {
                (RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"title" => {
                    feed.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"link" => {
                    feed.link.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (
                    ResolveResult::Bound(Namespace(ns)),
                    event @ (Event::Start(_) | Event::Empty(_)),
                ) if let Some(handler) = ns::feed_handler(ns) => {
                    handler.handle_start(reader, event, feed, version, alloc)?;
                }
                (_, Event::Start(tag)) => {
                    reader.read_to_end(tag.name()).map_err(ParserError::Xml)?;
                }
                (_, Event::End(tag)) if tag.name() == name => {
                    return Ok(());
                }
                (_, Event::Eof) => return Err(ParserError::UNCLOSED_TAG),
                _ => {}
            }
        }
    }
}

pub enum Parser {
    OutsideChannel,
    InsideChannel,
}
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for Parser
where
    A: Allocator,
{
    fn try_from_root(
        root: BytesStart<'src>,
        reader: &NsReader<&'src [u8]>,
        _: XmlVersion,
    ) -> Result<Self, TryFromRootError<'src>> {
        if let (RDF, name) = reader.resolver().resolve_element(root.name())
            && name.as_ref() == b"RDF"
        {
            Ok(Self::OutsideChannel)
        } else {
            Err(TryFromRootError::UnknownRoot(root))
        }
    }

    fn handle_event<F>(
        self,
        reader: &mut NsReader<&'src [u8]>,
        event: Event<'src>,
        state: &mut PartialFeed<'alloc, 'src, A>,
        mut cb: F,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(xml::Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        match (self, reader.resolver().resolve_event(event)) {
            (step @ Self::OutsideChannel, (RSS, Event::Start(tag)))
                if tag.local_name().as_ref() == b"channel" =>
            {
                RdfChannel::parse_tag_into(state, reader, tag.name(), version, alloc).map(|_| step)
            }
            (step @ Self::OutsideChannel, (RSS, Event::Start(tag)))
                if tag.local_name().as_ref() == b"item" =>
            {
                RdfItemHandler::parse_tag_into(&mut cb, reader, tag.name(), version, alloc)
                    .map(|_| step)
            }
            (step, (_, Event::Start(tag))) => reader
                .read_to_end(tag.name())
                .map(|_| step)
                .map_err(ParserError::Xml),
            (step, _) => Ok(step),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc,
            borrow::Cow,
            xml::{
                Feed, SkipDays, SkipHours,
                fmt::tests::test_parser_ns,
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::vec::Vec,
        jiff::Span,
    };

    #[test]
    fn test_rdf_parser_sample() -> Result<(), TestParserError<'static>> {
        test_parser::<_, Parser, _>(
            include_str!("./sample.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"XML.com")),
                link: Some(Cow::Borrowed(b"http://xml.com/pub")),
                last_update: None,
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: Span::new(),
                frequency: None,
            },
            [
                Entry {
                    title: Some(Cow::Borrowed(b"Processing Inclusions with XSLT")),
                    link: Some(Cow::Borrowed(b"http://xml.com/pub/2000/08/09/xslt/xslt.html")),
                    description: Some(Cow::Borrowed(b"\n     Processing document inclusions with general XML tools can be \n     problematic. This article proposes a way of preserving inclusion \n     information through SAX-based processing.\n    ")),
                    id: None,
                    pub_date: None,
                    enclosures: Vec::new_in(&alloc::Dummy),
                },
                Entry {
                    title: Some(Cow::Borrowed(b"Putting RDF to Work")),
                    link: Some(Cow::Borrowed(b"http://xml.com/pub/2000/08/09/rdfdb/index.html")),
                    description: Some(Cow::Borrowed(b"\n     Tool and API support for the Resource Description Framework \n     is slowly coming of age. Edd Dumbill takes a look at RDFDB, \n     one of the most exciting new RDF toolkits.\n    ")),
                    id: None,
                    pub_date: None,
                    enclosures: Vec::new_in(&alloc::Dummy),
                }
            ],
            &alloc::Dummy,
        )
    }

    #[test]
    fn test_rdf_parser_ns() -> Result<(), TestParserError<'static>> {
        test_parser_ns::<Parser>(include_str!("./ns.xml"))
    }
}
