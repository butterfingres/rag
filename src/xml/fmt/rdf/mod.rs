use {
    crate::xml::{
        self, Entry, ParserError, PartialEntry, PartialFeed, TryFromRootError,
        parser::{Content, ParseTagInto, TagParser},
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::QName,
        reader::NsReader,
    },
};

mod ns {
    use quick_xml::name::{Namespace, ResolveResult};

    pub const RDF: ResolveResult<'static> =
        ResolveResult::Bound(Namespace(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#"));
    pub const RSS: ResolveResult<'static> =
        ResolveResult::Bound(Namespace(b"http://purl.org/rss/1.0/"));
}

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
                (ns::RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"title" => {
                    entry.title = Some(Content.parse_tag(reader, tag.name(), version, alloc)?);
                }
                (ns::RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"description" => {
                    entry.content.try_replace_or_skip::<false, _, _>(
                        Content.map(Some),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns::RSS, Event::Start(tag)) if tag.local_name().as_ref() == b"link" => {
                    entry.link.try_replace_or_skip::<false, _, _>(
                        Content.map(Some),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
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
        if let (ns::RDF, name) = reader.resolver().resolve_element(root.name())
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
        match event {
            Event::Start(tag) => match (self, reader.resolver().resolve_element(tag.name())) {
                (Self::OutsideChannel, (ns::RSS, name)) if name.as_ref() == b"channel" => {
                    Ok(Self::InsideChannel)
                }
                (step @ Self::InsideChannel, (ns::RSS, name)) if name.as_ref() == b"title" => {
                    state.title = Content
                        .parse_tag(reader, tag.name(), version, alloc)
                        .map(Some)?;

                    Ok(step)
                }
                (step @ Self::InsideChannel, (ns::RSS, name)) if name.as_ref() == b"link" => {
                    state.link.try_replace_or_skip::<false, _, _>(
                        Content.map(Some),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;

                    Ok(step)
                }
                (step @ Self::OutsideChannel, (ns::RSS, name)) if name.as_ref() == b"item" => {
                    RdfItemHandler::parse_tag_into(&mut cb, reader, tag.name(), version, alloc)
                        .map(|_| step)
                }
                (step, _) => {
                    reader.read_to_end(tag.name())?;
                    Ok(step)
                }
            },
            Event::End(tag) => match (self, reader.resolver().resolve_element(tag.name())) {
                (Self::InsideChannel, (ns::RSS, name)) if name.as_ref() == b"channel" => {
                    Ok(Self::OutsideChannel)
                }
                (step, _) => Ok(step),
            },
            _ => Ok(self),
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
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::vec::Vec,
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
                ttl: None,
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
}
