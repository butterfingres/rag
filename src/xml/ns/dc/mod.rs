//! Dublin core parser.
//!
//! See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>.

use {
    crate::{
        num::parse,
        xml::{
            ParserError, PartialEntry, PartialFeed, Replaceable,
            ns::HandleStart,
            parser::{Content, TagParser as _},
        },
    },
    allocator_api2::alloc::Allocator,
    jiff::{Timestamp, civil::datetime, fmt::temporal::DateTimeParser, tz::TimeZone},
    lazy_regex::bytes_regex_captures,
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
        reader: &mut NsReader<&'src [u8]>,
        start: Event<'src>,
        item: &mut PartialEntry<'alloc, 'src, A>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        match start {
            Event::Start(tag) if tag.local_name().as_ref() == b"date" => {
                item.updated.try_replace_with(|| {
                    Content
                        .flat_map(parse_date)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }
            Event::Start(tag) if tag.local_name().as_ref() == b"description" => {
                item.content.try_replace_with(|| {
                    Content
                        .map(Replaceable::new_replaceable)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }
            Event::Start(tag) if tag.local_name().as_ref() == b"title" => {
                item.title.try_replace_with(|| {
                    Content
                        .map(Replaceable::new_replaceable)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }
            // we should prefer native identifier types
            Event::Start(tag) if tag.local_name().as_ref() == b"identifier" => {
                item.id.try_replace_with(|| {
                    Content
                        .map(Replaceable::new_replaceable)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }

            Event::Start(tag) => {
                reader.read_to_end(tag.name())?;
            }
            _ => {}
        }

        Ok(())
    }
}
impl<'alloc, 'src, A> HandleStart<'alloc, 'src, PartialFeed<'alloc, 'src, A>, A> for Parser
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
        match start {
            Event::Start(tag) if tag.local_name().as_ref() == b"date" => {
                feed.last_update.try_replace_with(|| {
                    Content
                        .flat_map(parse_date)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }
            Event::Start(tag) if tag.local_name().as_ref() == b"title" => {
                feed.title.try_replace_with(|| {
                    Content
                        .map(Replaceable::new_replaceable)
                        .map(|replaceable| replaceable.map(Some))
                        .parse_tag(reader, tag.name(), version, alloc)
                })?;
            }

            Event::Start(tag) => {
                reader.read_to_end(tag.name())?;
            }
            _ => {}
        }

        Ok(())
    }
}

/// Parse the dublin core date.
///
/// See <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#date>.
fn parse_date<T>(date: T) -> Result<Replaceable<Timestamp>, ParserError>
where
    T: AsRef<[u8]>,
{
    let date = date.as_ref();

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

    let ts = if let Some((_, year, month)) =
        bytes_regex_captures!(r#"^([0-9]{4})(-[0-9]{2})?$"#, date)
    {
        let year =
            i16::try_from(parse::<_, u16>(year)?).map_err(|_| ParserError::DateOutOfRange)?;
        let month = if let [b'-', month @ ..] = month {
            i8::try_from(parse::<_, u8>(month)?).map_err(|_| ParserError::DateOutOfRange)?
        } else {
            01
        };
        datetime(year, month, 01, 00, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    } else {
        DateTimeParser::new()
            .parse_datetime(date)
            .and_then(|dt| dt.to_zoned(TimeZone::UTC))
            .map(|zoned| zoned.timestamp())
            .or_else(|_| DateTimeParser::new().parse_timestamp(date))?
    };

    Ok(Replaceable {
        replaceable,
        data: ts,
    })
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc::Dummy,
            borrow::Cow,
            xml::{
                Entry, Feed, SkipDays, SkipHours,
                ns::tests::{test_feed_parser, test_item_parser},
            },
        },
        allocator_api2::vec::Vec,
        arrayvec::ArrayVec,
        jiff::{Span, civil::datetime},
        std::iter,
    };

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
        test_parse_date_y,
        b"2000",
        datetime(2000, 01, 01, 00, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    );
    test_parse_date!(
        test_parse_date_ym,
        b"2000-01",
        datetime(2000, 01, 01, 00, 00, 00, 00)
            .to_zoned(TimeZone::UTC)?
            .timestamp()
    );
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

    #[test]
    fn test_dc_parser_item() -> Result<(), ParserError> {
        let alloc = Dummy;
        test_item_parser(
            &Parser,
            include_str!("./item.xml"),
            Entry {
                title: Some(Cow::Borrowed(b"first entry")),
                link: None,
                description: Some(Cow::Borrowed(b"example description")),
                id: Some(Cow::Borrowed(b"1")),
                pub_date: Some(
                    datetime(2000, 01, 01, 00, 00, 00, 00)
                        .to_zoned(TimeZone::UTC)?
                        .timestamp(),
                ),
                enclosures: Vec::new_in(&alloc),
            },
            &alloc,
        )
    }

    #[test]
    fn test_dc_parser_channel() -> Result<(), ParserError> {
        let alloc = Dummy;
        test_feed_parser(
            &Parser,
            include_str!("./channel.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"example feed")),
                link: None,
                skip_days: SkipDays::default(),
                skip_hours: SkipHours::default(),
                ttl: Span::new(),
                frequency: None,
                last_update: Some(
                    datetime(2001, 01, 02, 00, 00, 00, 00)
                        .to_zoned(TimeZone::UTC)?
                        .timestamp(),
                ),
            },
            &alloc,
        )
    }
}
