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
        media::NS => Some(&media::Parser),
        _ => None,
    }
}
