use {
    crate::xml::{ParserError, PartialFeed, ns::HandleStart},
    allocator_api2::alloc::Allocator,
    quick_xml::{
        XmlVersion,
        events::Event,
        name::{Namespace, ResolveResult},
        reader::NsReader,
    },
};

pub const NS: &[u8] = b"http://search.yahoo.com/mrss/";

fn handle_start<'alloc, 'src, A>(
    reader: &mut NsReader<&'src [u8]>,
    start: Event<'src>,
    feed: &mut PartialFeed<'alloc, 'src, A>,
    version: XmlVersion,
    alloc: &'alloc A,
    recursed: bool,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    match start {
        Event::Start(tag) if tag.local_name().into_inner() == b"group" && !recursed => loop {
            match reader.read_resolved_event()? {
                (_, Event::End(end_tag)) if tag.name() == end_tag.name() => break,
                (_, Event::Eof) => return Err(ParserError::MissingRoot),
                (ResolveResult::Bound(Namespace(NS)), tag) => {
                    handle_start(reader, tag, feed, version, alloc, true)?;
                }
                _ => {}
            }
        },
        Event::Start(tag) => {
            reader.read_to_end(tag.name())?;
        }
        Event::Empty(_) => {}
        _ => {}
    }

    Ok(())
}

pub struct Parser;
impl<'alloc, 'src, A> HandleStart<'alloc, 'src, A> for Parser
where
    A: Allocator,
{
    fn handle_start(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        start: Event<'src>,
        feed: &mut PartialFeed<'alloc, 'src, A>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        handle_start(reader, start, feed, version, alloc, false)
    }
}
