use {
    crate::xml::{
        self, HandleElementInto, OptionHandler, ParserError, PartialFeed, ReplaceableHandler,
        TryFromRootError,
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
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

pub enum RdfParser {
    OutsideChannel,
    InsideChannel,
}
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for RdfParser
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
        _cb: F,
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
                    OptionHandler::<_>::handle_element_into(
                        &mut state.title,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )
                    .map(|_| step)
                }
                (step @ Self::InsideChannel, (ns::RSS, name)) if name.as_ref() == b"link" => {
                    OptionHandler::<ReplaceableHandler<false, _>, _>::handle_element_into(
                        &mut state.link,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )
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
    };

    #[test]
    fn test_rdf_parser_sample() -> Result<(), TestParserError<'static>> {
        test_parser::<_, RdfParser, _>(
            include_str!("./sample.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"XML.com")),
                link: Some(Cow::Borrowed(b"http://xml.com/pub")),
                last_update: None,
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: None,
            },
            [],
            &alloc::Dummy,
        )
    }
}
