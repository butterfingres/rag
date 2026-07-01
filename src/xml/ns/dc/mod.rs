//! Dublin core parser.
//!
//! See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>.

use {
    crate::xml::{ParserError, PartialEntry, Replaceable, ns::HandleStart},
    allocator_api2::alloc::Allocator,
    jiff::{Timestamp, fmt::temporal::DateTimeParser},
    memchr::memchr,
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

/// Parse the dublin core date.
///
/// See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#date>.
fn _parse_date(date: &[u8]) -> Result<Replaceable<Timestamp>, ParserError> {
    // If the timestamp contains a slash then it is ambiguous because
    // the publishing date might be anywhere between the range.
    let (replaceable, date) = memchr(b'/', date)
        .map(|idx| {
            let l = &date[..idx];
            let r = &date[idx + 1..];
            // We prefer the right (end time) because we can use
            // the latest timestamp in If-Modified-Since.
            (true, if r.is_empty() { l } else { r })
        })
        .unwrap_or((false, date));

    // We use the RFC 9557 parser because the Dublin Core specs don't
    // say that the timestamp won't contain timezones and the
    // [jiff::civil::DateTime] will return errors for timezones.
    Ok(Replaceable {
        replaceable,
        data: DateTimeParser::new().parse_zoned(date)?.timestamp(),
    })
}
