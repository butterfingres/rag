use {
    crate::xml::{
        ParserError, PartialEntry, get_attribute_when,
        ns::HandleStart,
        parser::{Content, TagParser as _},
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, ResolveResult},
        reader::NsReader,
    },
};

pub const NS: &[u8] = b"http://search.yahoo.com/mrss/";

fn handle_url_attribute<'alloc, 'src, A>(
    reader: &NsReader<&'src [u8]>,
    start: BytesStart<'src>,
    item: &mut PartialEntry<'alloc, 'src, A>,
    version: XmlVersion,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    if let Some(url) = get_attribute_when(
        &start,
        |_| Ok(true),
        |attr| matches!(reader.resolver().resolve(attr.key, true), (ResolveResult::Bound(Namespace(NS)), name) if name.as_ref() == b"url"),
        version,
        alloc,
    )? {
        item.enclosures.push(url);
    }

    Ok(())
}

fn handle_start<'alloc, 'src, A>(
    reader: &mut NsReader<&'src [u8]>,
    start: Event<'src>,
    item: &mut PartialEntry<'alloc, 'src, A>,
    version: XmlVersion,
    alloc: &'alloc A,
    recursed: bool,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    match start {
        Event::Start(tag) if tag.local_name().as_ref() == b"group" && !recursed => loop {
            match reader.read_resolved_event()? {
                (_, Event::End(end_tag)) if tag.name() == end_tag.name() => break,
                (_, Event::Eof) => return Err(ParserError::MissingRoot),
                (ResolveResult::Bound(Namespace(NS)), tag) => {
                    handle_start(reader, tag, item, version, alloc, true)?;
                }
                _ => {}
            }
        },

        Event::Start(tag) if let b"content" | b"player" = tag.local_name().as_ref() => {
            reader.read_to_end(tag.name())?;
            handle_url_attribute(reader, tag, item, version, alloc)?;
        }
        Event::Empty(tag) if let b"content" | b"player" = tag.local_name().as_ref() => {
            handle_url_attribute(reader, tag, item, version, alloc)?;
        }

        // TODO: handle type
        Event::Start(tag) if tag.local_name().as_ref() == b"title" => {
            item.title = Some(Content.parse_tag(reader, tag.name(), version, alloc)?);
        }
        Event::Start(tag) if tag.local_name().as_ref() == b"description" => {
            item.content.try_replace_or_skip::<false, _, _>(
                Content.map(Some),
                reader,
                tag.name(),
                version,
                alloc,
            )?;
        }

        Event::Start(tag) => {
            reader.read_to_end(tag.name())?;
        }
        Event::Empty(_) => {}
        _ => {}
    }

    Ok(())
}

pub struct Parser;
impl<'alloc, 'src, A> HandleStart<'alloc, 'src, PartialEntry<'alloc, 'src, A>, A> for Parser
where
    A: Allocator,
{
    fn handle_start(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        start: Event<'src>,
        item: &mut PartialEntry<'alloc, 'src, A>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        handle_start(reader, start, item, version, alloc, false)
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            borrow::Cow,
            xml::{Entry, ns::tests::test_item_parser},
        },
        allocator_api2::{alloc::Global, boxed::Box, vec},
        bump_scope::Bump,
    };

    #[test]
    fn test_media_parser() -> Result<(), ParserError> {
        let mut alloc = Bump::<Global>::try_new()?;

        test_item_parser(
            &Parser,
            include_str!("./item.xml"),
            Entry {
                title: Some(Cow::Borrowed(b"hello world")),
                link: None,
                description: Some(Cow::Borrowed(b"test description")),
                id: None,
                pub_date: None,
                enclosures: vec![in &alloc; Box::slice(Box::new_in(*b"https://example.com/hello_world.mp3", &alloc))],
            },
            &alloc,
        )?;
        alloc.reset_to_start();

        test_item_parser(
            &Parser,
            include_str!("./group.xml"),
            Entry {
                title: Some(Cow::Borrowed(b"hello world")),
                link: None,
                description: Some(Cow::Borrowed(b"test description")),
                id: None,
                pub_date: None,
                enclosures: vec![in &alloc; Box::slice(Box::new_in(*b"https://example.com/hello_world.mp3", &alloc))],
            },
            &alloc,
        )
    }
}
