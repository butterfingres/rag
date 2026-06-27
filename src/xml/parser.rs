use {
    crate::xml::ParserError,
    allocator_api2::alloc::Allocator,
    quick_xml::{XmlVersion, name::QName, reader::NsReader},
};

pub trait TagParser<'alloc, 'src, A>
where
    A: Allocator,
{
    type Output;

    fn parse_tag(
        &self,
        _: &mut NsReader<&'src [u8]>,
        _: QName<'_>,
        _: XmlVersion,
        _: &'alloc A,
    ) -> Result<Self::Output, ParserError>;
}
