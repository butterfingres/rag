use {
    super::*,
    crate::{
        alloc, tz,
        xml::tests::{TestParserError, test_parser},
    },
    allocator_api2::{alloc::Global, vec},
    jiff::{civil::datetime, tz::TimeZone},
};

#[test]
fn test_rss_parser_all() -> Result<(), TestParserError<'static>> {
    test_parser::<_, Step, _>(
        include_str!("./all.xml"),
        Channel {
            title: Some(Cow::Borrowed(b"example feed")),
            link: Some(Cow::Borrowed(b"https://example.com/rss")),
            modify_date: Some(Replaceable {
                data: datetime(2023, 07, 21, 09, 04, 00, 00)
                    .to_zoned(tz::EDT)?
                    .timestamp()
                    .into(),
                replaceable: false,
            }),
            skip_hours: SkipHours::new([0b0111]),
            skip_days: SkipDays::new([0b0111]),
        },
        [Entry {
            title: Some(Cow::Borrowed(b"entry 1")),
            link: Some(Cow::Borrowed(b"https://example.com/entry_1")),
            description: Some(Cow::Borrowed(b"the first entry")),
            pub_date: datetime(2003, 06, 20, 09, 00, 00, 00)
                .to_zoned(TimeZone::UTC)?
                .timestamp()
                .into(),
            enclosures: vec![in &Global; Enclosure {
                tag: BytesStart::from_content(r#"enclosure url="https://example.com/entry_1.mp3""#, "enclosure".len()),
                enclosure: 15..46,
            }],
        }],
        &Global,
    )
}

#[test]
fn test_rss_parser_alt() -> Result<(), TestParserError<'static>> {
    test_parser::<_, Step, _>(
        include_str!("./alt.xml"),
        Channel {
            title: Some(Cow::Borrowed(b"example feed")),
            link: Some(Cow::Borrowed(b"https://example.com/rss")),
            modify_date: Some(Replaceable {
                data: datetime(2023, 07, 21, 09, 04, 00, 00)
                    .to_zoned(tz::EDT)?
                    .timestamp()
                    .into(),
                replaceable: false,
            }),
            skip_hours: SkipHours::default(),
            skip_days: SkipDays::new([0b0111_1111]),
        },
        [],
        &alloc::Dummy,
    )
}
