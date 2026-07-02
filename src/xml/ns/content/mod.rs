//! Rss 1.0 content parser.
//!
//! See <https://web.resource.org/rss/1.0/modules/content/>.

use {
    crate::xml::{
        ParserError, PartialEntry, Replaceable,
        ns::HandleStart,
        parser::{Content, TagParser as _},
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{XmlVersion, events::Event, reader::NsReader},
};

pub const NS: &[u8] = b"http://purl.org/rss/1.0/modules/content/";

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
        match start {
            Event::Start(tag) if tag.local_name().as_ref() == b"encoded" => {
                item.content.try_replace_with(|| {
                    Content
                        .map(Replaceable::new_irreplaceable)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc::Dummy,
            borrow::Cow,
            xml::{Entry, ns::tests::test_item_parser},
        },
        allocator_api2::vec::Vec,
    };

    #[test]
    fn test_dc_parser_item() -> Result<(), ParserError> {
        let alloc = Dummy;
        test_item_parser(
            &Parser,
            include_str!("./item.xml"),
            Entry {
                title: None,
                link: None,
                description: Some(Cow::Borrowed(b"content")),
                id: None,
                pub_date: None,
                enclosures: Vec::new_in(&alloc),
            },
            &alloc,
        )
    }
}
