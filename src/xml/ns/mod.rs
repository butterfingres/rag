pub mod content;
pub mod dc;
pub mod media;
pub mod sy;

use {
    crate::xml::{ParserError, PartialEntry, PartialFeed},
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
    ///
    /// If the [Event] is an [Event::Start], the reader should be read
    /// to the end of the element.
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
        content::NS => Some(&content::Parser),
        dc::NS => Some(&dc::Parser),
        media::NS => Some(&media::Parser),
        _ => None,
    }
}

pub const fn namespace_feed_handler<'alloc, 'src, A>(
    ns: &[u8],
) -> Option<&'static dyn HandleStart<'alloc, 'src, PartialFeed<'alloc, 'src, A>, A>>
where
    A: Allocator,
{
    match ns {
        dc::NS => Some(&dc::Parser),
        sy::NS => Some(&sy::Parser),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::xml::{Entry, Feed, PartialFeed, get_header},
        std::fmt::Debug,
    };

    pub fn test_parser<'alloc, 'src, T, U, V, A>(
        parser: &T,
        input: &'src str,
        mut buffer: U,
        output: V,
        alloc: &'alloc A,
    ) -> Result<(), ParserError>
    where
        T: HandleStart<'alloc, 'src, U, A>,
        U: TryInto<V>,
        <U as TryInto<V>>::Error: Into<ParserError>,
        V: Debug + PartialEq,
        A: Allocator,
    {
        let mut reader = NsReader::from_str(input);
        let (version, root) = get_header(&mut reader)?;

        loop {
            match reader.read_event()? {
                Event::Eof => break,
                Event::End(tag) if tag.name() == root.name() => break,
                event => parser.handle_start(&mut reader, event, &mut buffer, version, alloc)?,
            }
        }

        assert_eq!(
            <U as TryInto<V>>::try_into(buffer)
                .map_err(<<U as TryInto<V>>::Error as Into<ParserError>>::into)?,
            output
        );

        Ok(())
    }

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
        test_parser(parser, input, PartialEntry::new_in(alloc), output, alloc)
    }

    pub fn test_feed_parser<'alloc, 'src, T, A>(
        parser: &T,
        input: &'src str,
        output: Feed<'alloc, 'src, A>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError>
    where
        T: HandleStart<'alloc, 'src, PartialFeed<'alloc, 'src, A>, A>,
        A: Allocator,
    {
        test_parser(parser, input, PartialFeed::default(), output, alloc)
    }
}
