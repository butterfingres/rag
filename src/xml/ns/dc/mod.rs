//! Dublin core parser.
//!
//! See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>.

use {
    crate::xml::{ParserError, PartialEntry, Replaceable, ns::HandleStart},
    allocator_api2::alloc::Allocator,
    chrono::DateTime,
    jiff::{Timestamp, fmt::temporal::DateTimeParser, tz::TimeZone},
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
#[allow(dead_code, reason = "this function is currently only used in tests")]
fn parse_date(date: &[u8]) -> Result<Replaceable<Timestamp>, ParserError> {
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

    let date = str::from_utf8(date)?;
    // Neither [chrono] or [jiff] have complete ISO-8601 parsers, so
    // we must use both.
    let timestamp = match DateTime::parse_from_rfc3339(date) {
        Ok(dt) => Timestamp::from_second(dt.timestamp())?,
        Err(err) if err.kind() == chrono::format::ParseErrorKind::TooShort => DateTimeParser::new()
            .parse_datetime(&date)?
            .to_zoned(TimeZone::UTC)?
            .timestamp(),
        Err(err) => return Err(ParserError::ChronoParse(err)),
    };

    Ok(Replaceable {
        replaceable,
        data: timestamp,
    })
}

#[cfg(test)]
mod tests {
    use {super::*, arrayvec::ArrayVec, jiff::civil::datetime, std::iter};

    const RANGE_TS: &[u8] = b"2001-01-01";

    fn test_parse_date<const BUF: usize>(
        input: &[u8],
        output: Timestamp,
    ) -> Result<(), ParserError> {
        let mut buf = ArrayVec::<_, BUF>::new();
        buf.try_extend_from_slice(input).unwrap();
        assert_eq!(
            parse_date(&buf)?,
            Replaceable {
                data: output,
                replaceable: false
            }
        );

        buf.push(b'/');
        assert_eq!(
            parse_date(&buf)?,
            Replaceable {
                data: output,
                replaceable: true
            }
        );

        buf.clear();
        buf.extend(iter::once(b'/').chain(input.iter().copied()));
        assert_eq!(
            parse_date(&buf)?,
            Replaceable {
                data: output,
                replaceable: true
            }
        );

        buf.clear();
        buf.extend(
            RANGE_TS
                .iter()
                .copied()
                .chain(iter::once(b'/'))
                .chain(input.iter().copied()),
        );
        assert_eq!(
            parse_date(&buf)?,
            Replaceable {
                data: output,
                replaceable: true
            },
            "the right side should be favored"
        );

        buf.clear();
        buf.extend(
            input
                .iter()
                .copied()
                .chain(iter::once(b'/'))
                .chain(RANGE_TS.iter().copied()),
        );
        assert_eq!(
            parse_date(&buf)?,
            Replaceable {
                data: datetime(2001, 01, 01, 00, 00, 00, 00)
                    .to_zoned(TimeZone::UTC)?
                    .timestamp(),
                replaceable: true
            },
            "the right side should be favored"
        );

        Ok(())
    }

    macro_rules! test_parse_date {
        ($name:ident, $input:literal, $output:expr) => {
            #[test]
            fn $name() -> Result<(), ParserError> {
                const INPUT: &[u8] = $input;
                test_parse_date::<{ INPUT.len() + 1 + RANGE_TS.len() }>(INPUT, $output)
            }
        };
    }
    test_parse_date!(
        test_parse_date_ymd,
        b"2000-01-01",
        datetime(2000, 01, 01, 00, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    );
    test_parse_date!(
        test_parse_date_ymd_dt,
        b"2000-01-01T12:00:00",
        datetime(2000, 01, 01, 12, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    );
    test_parse_date!(
        test_parse_date_ymd_dt_timezone,
        b"2000-01-01T12:00:00Z",
        datetime(2000, 01, 01, 12, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    );
}
