use {
    crate::{
        borrow::Cow,
        xml::{
            self, Entry, HandleElementInto, OptionHandler, ParserError, Replaceable,
            Rfc3339Timestamp, TryFromRootError, get_attribute_when,
        },
    },
    allocator_api2::{alloc::Allocator, boxed::Box},
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, ResolveResult},
        reader::NsReader,
    },
    std::fmt::{self, Debug, Formatter},
};

const NS: &[u8] = b"http://www.w3.org/2005/Atom";

pub struct Feed<'alloc, 'src, A>
where
    A: Allocator,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Replaceable<Box<[u8], &'alloc A>>>,
    update: Option<Rfc3339Timestamp>,
}
impl<A> Debug for Feed<'_, '_, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Feed").field("title", &self.title).finish()
    }
}
impl<A> Default for Feed<'_, '_, A>
where
    A: Allocator,
{
    fn default() -> Self {
        Self {
            title: None,
            link: None,
            update: None,
        }
    }
}
impl<A1, A2> PartialEq<Feed<'_, '_, A2>> for Feed<'_, '_, A1>
where
    A1: Allocator,
    A2: Allocator,
{
    fn eq(&self, r: &Feed<'_, '_, A2>) -> bool {
        self.title.as_deref() == r.title.as_deref()
    }
}
impl<'alloc, 'src, A> Feed<'alloc, 'src, A>
where
    A: Allocator,
{
    fn handle_link(
        &mut self,
        link: &BytesStart<'src>,
        reader: &NsReader<&'src [u8]>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut replaceable = true;
        if let Some(Replaceable {
            replaceable: true, ..
        })
        | None = self.link
            && let Some(href) = get_attribute_when(
                &link,
                |attr| {
                    if let (ResolveResult::Bound(Namespace(NS)), name) =
                        reader.resolver().resolve_attribute(attr.key)
                        && name.as_ref() == b"rel"
                        && *attr.value == *b"alternate"
                    {
                        replaceable = false;
                    }

                    true
                },
                |attr| matches!(reader.resolver().resolve_attribute(attr.key), (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"href"),
                version,
                alloc,
            )?
        {
            self.link = Some(Replaceable {
                replaceable,
                data: href,
            });
        }
        Ok(())
    }
}

pub struct AtomParser;
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for AtomParser
where
    A: Allocator + 'alloc,
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
        version: XmlVersion,
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
                        version,
                        alloc,
                    )?;
                }
                (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"updated" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut state.update,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"link" => {
                    state.handle_link(&tag, reader, version, alloc)?;
                    reader.read_to_end(tag.name())?;
                }
                _ => {
                    reader.read_to_end(tag.name())?;
                }
            },
            Event::Empty(tag) => match reader.resolver().resolve_element(tag.name()) {
                (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"link" => {
                    state.handle_link(&tag, reader, version, alloc)?;
                }
                _ => {}
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
            tz,
            xml::tests::{TestParserError, test_parser},
        },
        allocator_api2::alloc::Global,
        bump_scope::Bump,
        jiff::civil::datetime,
    };

    #[test]
    fn test_atom_parser_all() -> Result<(), TestParserError<'static>> {
        let alloc = Bump::<Global>::try_new()?;
        test_parser::<_, AtomParser, _>(
            include_str!("./all.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"test feed")),
                link: Some(Replaceable {
                    replaceable: false,
                    data: Box::slice(Box::new_in(*b"https://example.com/entry_1.mp3", &alloc)),
                }),
                // 2003-12-13T18:30:02Z
                update: Some(
                    datetime(2003, 12, 13, 18, 30, 02, 00)
                        .to_zoned(tz::Z)?
                        .timestamp()
                        .into(),
                ),
            },
            [],
            &alloc,
        )
    }
}
