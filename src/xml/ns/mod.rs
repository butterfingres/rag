pub mod dc;
pub mod media;

use {
    crate::xml::{ParserError, PartialEntry},
    allocator_api2::alloc::Allocator,
    quick_xml::{XmlVersion, events::Event, reader::NsReader},
};

pub trait HandleStart<'alloc, 'src, T, A>
where
    A: Allocator,
{
    /// Handle a start [Event].
    ///
    /// This only needs to handle [Event::Start] and [Event::Empty].
    fn handle_start(
        &self,
        _: &mut NsReader<&'src [u8]>,
        _: Event<'src>,
        _: &mut T,
        _: XmlVersion,
        _: &'alloc A,
    ) -> Result<(), ParserError>;
}

pub const fn namespace_item_handler<'alloc, 'src, A>(
    ns: &[u8],
) -> Option<&'static dyn HandleStart<'alloc, 'src, PartialEntry<'alloc, 'src, A>, A>>
where
    A: Allocator,
{
    match ns {
        dc::NS => Some(&dc::Parser),
        media::NS => Some(&media::Parser),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::xml::{Entry, get_header},
    };

    pub fn test_item_parser<'alloc, 'src, T, A>(
        parser: &T,
        input: &'src str,
        output: Entry<'alloc, 'src, A>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError>
    where
        T: HandleStart<'alloc, 'src, PartialEntry<'alloc, 'src, A>, A>,
        A: Allocator,
    {
        let mut reader = NsReader::from_str(input);
        let (version, root) = get_header(&mut reader)?;

        let mut item = PartialEntry::new_in(alloc);
        loop {
            match reader.read_event()? {
                Event::Eof => break,
                Event::End(tag) if tag.name() == root.name() => break,
                event => parser.handle_start(&mut reader, event, &mut item, version, alloc)?,
            }
        }

        assert_eq!(Entry::from(item), output);

        Ok(())
    }
}
