use {
    crate::{
        borrow::Cow,
        xml::{
            self, Entry, HandleElementInto, OptionHandler, ParserError, Rfc3339Timestamp,
            TryFromRootError,
        },
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        events::{BytesStart, Event},
        name::{Namespace, ResolveResult},
        reader::NsReader,
    },
    std::fmt::{self, Debug, Formatter},
};

const NS: &[u8] = b"http://www.w3.org/2005/Atom";

pub struct Feed<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    update: Option<Rfc3339Timestamp>,
}
impl<A> Debug for Feed<'_, '_, A>
where
    A: Allocator + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Feed").field("title", &self.title).finish()
    }
}
impl<A> Default for Feed<'_, '_, A>
where
    A: Allocator + ?Sized,
{
    fn default() -> Self {
        Self {
            title: None,
            update: None,
        }
    }
}
impl<A1, A2> PartialEq<Feed<'_, '_, A2>> for Feed<'_, '_, A1>
where
    A1: Allocator + ?Sized,
    A2: Allocator + ?Sized,
{
    fn eq(&self, r: &Feed<'_, '_, A2>) -> bool {
        self.title.as_deref() == r.title.as_deref()
    }
}

pub struct AtomParser;
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for AtomParser
where
    A: Allocator + ?Sized + 'alloc,
{
    type Reader = NsReader<&'src [u8]>;
    type State = Feed<'alloc, 'src, A>;

    fn try_from_root(
        root: BytesStart<'src>,
        reader: &Self::Reader,
    ) -> Result<Self, TryFromRootError<'src>> {
        if let (ResolveResult::Bound(Namespace(NS)), name) =
            reader.resolver().resolve_element(root.name())
            && name.as_ref() == b"feed"
        {
            Ok(Self)
        } else {
            Err(TryFromRootError::UnknownRoot(root))
        }
    }
    fn handle_event<F>(
        self,
        reader: &mut Self::Reader,
        event: Event<'src>,
        state: &mut Self::State,
        _: F,
        alloc: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        match event {
            Event::Start(tag) => match reader.resolver().resolve_element(tag.name()) {
                (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"title" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut state.title,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"updated" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut state.update,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                _ => {
                    reader.read_to_end(tag.name())?;
                }
            },
            _ => {}
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc, tz,
            xml::tests::{TestParserError, test_parser},
        },
        jiff::civil::datetime,
    };

    #[test]
    fn test_atom_parser_all() -> Result<(), TestParserError<'static>> {
        test_parser::<_, AtomParser, _>(
            include_str!("./all.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"test feed")),
                // 2003-12-13T18:30:02Z
                update: Some(
                    datetime(2003, 12, 13, 18, 30, 02, 00)
                        .to_zoned(tz::Z)?
                        .timestamp()
                        .into(),
                ),
            },
            [],
            &alloc::Dummy,
        )
    }
}
