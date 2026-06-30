//! Dublin core parser.
//!
//! See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>.

use {
    crate::xml::{ParserError, PartialEntry, ns::HandleStart},
    allocator_api2::alloc::Allocator,
    quick_xml::{XmlVersion, events::Event, reader::NsReader},
};

pub const NS: &[u8] = b"http://purl.org/dc/elements/1.1/";

pub struct Parser;
impl<'alloc, 'src, A> HandleStart<'alloc, 'src, PartialEntry<'alloc, 'src, A>, A> for Parser
where
    A: Allocator,
{
    fn handle_start(
        &self,
        _reader: &mut NsReader<&'src [u8]>,
        _start: Event<'src>,
        _item: &mut PartialEntry<'alloc, 'src, A>,
        _version: XmlVersion,
        _alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        Ok(())
    }
}
