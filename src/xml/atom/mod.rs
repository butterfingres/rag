use {
    crate::{
        borrow::Cow,
        xml::{
            self, Entry, HandleElementInto, OptionHandler, ParserError, PartialEntry, PartialFeed,
            Replaceable, ReplaceableHandler, Rfc3339TimestampHandler, TryFromRootError,
            get_attribute_when,
        },
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, QName},
        reader::NsReader,
    },
};

macro_rules! ns {
    () => {
        ::quick_xml::name::ResolveResult::Unbound
            | ::quick_xml::name::ResolveResult::Bound(Namespace(b"http://www.w3.org/2005/Atom"))
    };
}

fn handle_link<'alloc, 'src, A>(
    entry: &mut PartialEntry<'alloc, 'src, A>,
    link: &BytesStart<'src>,
    reader: &NsReader<&'src [u8]>,
    version: XmlVersion,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    #[derive(Default)]
    enum LinkType {
        Enclosure,
        Alternate,
        #[default]
        Other,
    }
    let mut found_rel = false;
    let mut ty = None;

    if let Some(href) = get_attribute_when(
        link,
        |attr| {
            if let (ns!(), name) = reader.resolver().resolve_attribute(attr.key)
                && name.as_ref() == b"rel"
            {
                found_rel = true;
                ty = Some(match attr.normalized_value(version)?.as_ref() {
                    "alternate" => LinkType::Alternate,
                    "enclosure" => LinkType::Enclosure,
                    _ => LinkType::Other,
                });
            }

            Ok(found_rel)
        },
        |attr| matches!(reader.resolver().resolve_attribute(attr.key), (ns!(), name) if name.as_ref() == b"href"),
        version,
        alloc,
    )? {
        match ty.unwrap_or_default() {
            LinkType::Enclosure => {
                entry.enclosures.push(href);
            }
            LinkType::Alternate => {
                entry.link = Some(Replaceable {
                    replaceable: false,
                    data: href,
                });
            }
            LinkType::Other
                if let None
                | Some(Replaceable {
                    replaceable: true, ..
                }) = entry.link =>
            {
                entry.link = Some(Replaceable {
                    replaceable: true,
                    data: href,
                });
            }
            _ => {}
        }
    }

    Ok(())
}

struct AtomEntry;
impl<'alloc, 'src, F, T, A> HandleElementInto<'alloc, 'src, A, F> for AtomEntry
where
    F: FnMut(Entry<'alloc, 'src, A>) -> T,
    T: Into<Result<(), ParserError>>,
    A: Allocator + 'alloc,
{
    fn handle_element_into(
        cb: &mut F,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut entry = PartialEntry::new_in(alloc);
        loop {
            match reader.read_resolved_event()? {
                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"id" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut entry.id,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"title" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut entry.title,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"content" => {
                    OptionHandler::<ReplaceableHandler<false, _>, _>::handle_element_into(
                        &mut entry.content,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"description" => {
                    OptionHandler::<ReplaceableHandler<true, _>, _>::handle_element_into(
                        &mut entry.content,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"updated" => {
                    OptionHandler::<Rfc3339TimestampHandler, _>::handle_element_into(
                        &mut entry.updated,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }

                (ns!(), Event::Start(tag)) if tag.local_name().as_ref() == b"link" => {
                    handle_link(&mut entry, &tag, reader, version, alloc)?;
                    reader.read_to_end(tag.name())?;
                }
                (ns!(), Event::Empty(tag)) if tag.local_name().as_ref() == b"link" => {
                    handle_link(&mut entry, &tag, reader, version, alloc)?;
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

fn feed_handle_link<'alloc, 'src, A>(
    feed: &mut PartialFeed<'alloc, 'src, A>,
    link: &BytesStart<'src>,
    reader: &NsReader<&'src [u8]>,
    version: XmlVersion,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    let mut replaceable = true;
    let mut found_rel = false;
    if let Some(Replaceable {
        replaceable: true, ..
    })
    | None = feed.link
        && let Some(href) = get_attribute_when(
            link,
            |attr| {
                if let (ns!(), name) = reader.resolver().resolve_attribute(attr.key)
                    && name.as_ref() == b"rel"
                    && *attr.value == *b"alternate"
                {
                    found_rel = true;
                    replaceable = false;
                }

                Ok(found_rel)
            },
            |attr| matches!(reader.resolver().resolve_attribute(attr.key), (ns!(), name) if name.as_ref() == b"href"),
            version,
            alloc,
        )?
    {
        feed.link = Some(Replaceable {
            replaceable,
            data: Cow::Owned(href.into()),
        });
    }
    Ok(())
}

pub struct AtomParser;
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for AtomParser
where
    A: Allocator + 'alloc,
{
    fn try_from_root(
        root: BytesStart<'src>,
        reader: &NsReader<&'src [u8]>,
        _: XmlVersion,
    ) -> Result<Self, TryFromRootError<'src>> {
        if let (ns!(), name) = reader.resolver().resolve_element(root.name())
            && name.as_ref() == b"feed"
        {
            Ok(Self)
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
            Event::Start(tag) => match reader.resolver().resolve_element(tag.name()) {
                (ns!(), name) if name.as_ref() == b"title" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut state.title,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), name) if name.as_ref() == b"updated" => {
                    OptionHandler::<ReplaceableHandler<false, Rfc3339TimestampHandler, _>, _>::handle_element_into(
                        &mut state.last_update,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ns!(), name) if name.as_ref() == b"link" => {
                    feed_handle_link(state, &tag, reader, version, alloc)?;
                    reader.read_to_end(tag.name())?;
                }

                (ns!(), name) if name.as_ref() == b"entry" => {
                    AtomEntry::handle_element_into(&mut cb, reader, tag.name(), version, alloc)?;
                }
                _ => {
                    reader.read_to_end(tag.name())?;
                }
            },
            Event::Empty(tag) => match reader.resolver().resolve_element(tag.name()) {
                (ns!(), name) if name.as_ref() == b"link" => {
                    feed_handle_link(state, &tag, reader, version, alloc)?;
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
            xml::{
                Feed, SkipDays, SkipHours,
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::{alloc::Global, boxed::Box, vec},
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
                link: Some(Cow::Borrowed(b"https://example.com")),
                // 2003-12-13T18:30:02Z
                last_update: Some(
                    datetime(2003, 12, 13, 18, 30, 02, 00)
                        .to_zoned(tz::Z)?
                        .timestamp()
                        .into(),
                ),
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: None,
            },
            [xml::Entry {
                title: Some(Cow::Borrowed(b"first entry")),
                link: Some(Cow::Borrowed(b"https://example.com/entry_1")),
                description: Some(Cow::Borrowed(b"contents of entry number 1")),
                id: Some(Cow::Borrowed(b"1")),
                // 2004-12-13T18:30:02Z
                pub_date: Some(
                    datetime(2004, 12, 13, 18, 30, 02, 00)
                        .to_zoned(tz::Z)?
                        .timestamp()
                        .into(),
                ),
                enclosures: vec![in &alloc;
                   Box::slice(Box::new_in(*b"https://example.com/entry_1.mp3", &alloc))
                ],
            }],
            &alloc,
        )
    }
}
