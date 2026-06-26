use {
    crate::xml::{self, ParserError, PartialFeed, TryFromRootError},
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
        _reader: &mut NsReader<&'src [u8]>,
        _event: Event<'src>,
        _state: &mut PartialFeed<'alloc, 'src, A>,
        _cb: F,
        _version: XmlVersion,
        _alloc: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(xml::Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc,
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
                title: None,
                link: None,
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
