use {
    crate::{
        borrow::Cow,
        xml::{
            self, Entry, ParserError, PartialEntry, PartialFeed, Replaceable, TryFromRootError,
            get_attribute_when, ns,
            parser::{Content, ParseTagInto, TagParser, rfc3339_timestamp},
        },
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, QName, ResolveResult},
        reader::NsReader,
    },
};

const NS: ResolveResult<'static> = ResolveResult::Bound(Namespace(b"http://www.w3.org/2005/Atom"));

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
            if let (ResolveResult::Unbound | NS, name) =
                reader.resolver().resolve_attribute(attr.key)
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
        |attr| matches!(reader.resolver().resolve_attribute(attr.key), (ResolveResult::Unbound | NS, name) if name.as_ref() == b"href"),
        version,
        alloc,
    )? {
        match ty.unwrap_or_default() {
            LinkType::Enclosure => {
                entry.enclosures.push(href);
            }
            LinkType::Alternate => {
                entry.link = Replaceable {
                    replaceable: false,
                    data: Some(Cow::Owned(href.into())),
                };
            }
            LinkType::Other
                if let Replaceable {
                    replaceable: true, ..
                } = entry.link =>
            {
                entry.link = Replaceable {
                    replaceable: true,
                    data: Some(Cow::Owned(href.into())),
                };
            }
            _ => {}
        }
    }

    Ok(())
}

struct AtomEntry;
impl<'alloc, 'src, F, T, A> ParseTagInto<'alloc, 'src, A, F> for AtomEntry
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
                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"id" => {
                    entry.id.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"title" => {
                    entry.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"content" => {
                    entry.content.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"description" => {
                    entry.content.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_replaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"updated" => {
                    entry.updated.try_replace_or_skip(
                        Content
                            .flat_map(rfc3339_timestamp)
                            .map(Some)
                            .map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }

                (NS, Event::Start(tag)) if tag.local_name().as_ref() == b"link" => {
                    handle_link(&mut entry, &tag, reader, version, alloc)?;
                    reader.read_to_end(tag.name())?;
                }
                (NS, Event::Empty(tag)) if tag.local_name().as_ref() == b"link" => {
                    handle_link(&mut entry, &tag, reader, version, alloc)?;
                }

                (
                    ResolveResult::Bound(Namespace(ns)),
                    start @ Event::Start(_) | start @ Event::Empty(_),
                ) if let Some(handler) = ns::item_handler(ns) => {
                    handler.handle_start(reader, start, &mut entry, version, alloc)?;
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
    if let Replaceable {
        replaceable: true, ..
    } = feed.link
        && let Some(href) = get_attribute_when(
            link,
            |attr| {
                if let (ResolveResult::Unbound | NS, name) =
                    reader.resolver().resolve_attribute(attr.key)
                    && name.as_ref() == b"rel"
                    && *attr.value == *b"alternate"
                {
                    found_rel = true;
                    replaceable = false;
                }

                Ok(found_rel)
            },
            |attr| matches!(reader.resolver().resolve_attribute(attr.key), (ResolveResult::Unbound | NS, name) if name.as_ref() == b"href"),
            version,
            alloc,
        )?
    {
        feed.link = Replaceable {
            replaceable,
            data: Some(Cow::Owned(href.into())),
        };
    }
    Ok(())
}

pub struct Parser;
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for Parser
where
    A: Allocator + 'alloc,
{
    fn try_from_root(
        root: BytesStart<'src>,
        reader: &NsReader<&'src [u8]>,
        _: XmlVersion,
    ) -> Result<Self, TryFromRootError<'src>> {
        if let (NS, name) = reader.resolver().resolve_element(root.name())
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
        match &event {
            Event::Start(tag) => match reader.resolver().resolve_element(tag.name()) {
                (NS, name) if name.as_ref() == b"title" => {
                    state.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (NS, name) if name.as_ref() == b"updated" => {
                    state.last_update.try_replace_or_skip(
                        Content
                            .flat_map(rfc3339_timestamp)
                            .map(Some)
                            .map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (NS, name) if name.as_ref() == b"link" => {
                    feed_handle_link(state, tag, reader, version, alloc)?;
                    reader.read_to_end(tag.name())?;
                }

                (NS, name) if name.as_ref() == b"entry" => {
                    AtomEntry::parse_tag_into(&mut cb, reader, tag.name(), version, alloc)?;
                }
                (ResolveResult::Bound(Namespace(ns)), _)
                    if let Some(handler) = ns::feed_handler(ns) =>
                {
                    handler.handle_start(reader, event, state, version, alloc)?;
                }
                _ => {
                    reader.read_to_end(tag.name())?;
                }
            },
            Event::Empty(tag) => match reader.resolver().resolve_element(tag.name()) {
                (NS, name) if name.as_ref() == b"link" => {
                    feed_handle_link(state, tag, reader, version, alloc)?;
                }
                (ResolveResult::Bound(Namespace(ns)), _)
                    if let Some(handler) = ns::feed_handler(ns) =>
                {
                    handler.handle_start(reader, event, state, version, alloc)?;
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
            alloc::with_bump,
            tz,
            xml::{
                Feed, SkipDays, SkipHours,
                fmt::tests::test_parser_ns,
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::{boxed::Box, vec},
        jiff::{Span, civil::datetime},
    };

    #[test]
    fn test_atom_parser_all() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
                include_str!("./all.xml"),
                Feed {
                    title: Some(Cow::Borrowed(b"test feed")),
                    link: Some(Cow::Borrowed(b"https://example.com")),
                    // 2003-12-13T18:30:02Z
                    last_update: Some(
                        datetime(2003, 12, 13, 18, 30, 02, 00)
                            .to_zoned(tz::Z)?
                            .timestamp(),
                    ),
                    skip_hours: SkipHours::default(),
                    skip_days: SkipDays::default(),
                    ttl: Span::new(),
                    frequency: None,
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
                            .timestamp(),
                    ),
                    enclosures: vec![in &alloc;
                       Box::slice(Box::new_in(*b"https://example.com/entry_1.mp3", &alloc))
                    ],
                }],
                &alloc,
            )
        })
    }

    #[test]
    fn test_atom_parser_ns() -> Result<(), TestParserError<'static>> {
        test_parser_ns::<Parser>(include_str!("./ns.xml"))
    }
}
