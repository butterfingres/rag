use {
    super::*,
    crate::{
        alloc, tz,
        xml::tests::{TestParserError, test_parser},
    },
    jiff::civil::datetime,
};

#[test]
fn test_parse_author() {
    let author = Author::<alloc::Dummy>::from(Cow::Borrowed(&b"foo@example.com"[..]));
    assert_eq!(author.email(), b"foo@example.com");
    assert_eq!(author.name(), None);
}

#[test]
fn test_parse_author_with_name() {
    let author = Author::<alloc::Dummy>::from(Cow::Borrowed(&b"foo@example.com (Bar Baz)"[..]));
    assert_eq!(author.email(), b"foo@example.com");
    assert_eq!(author.name(), Some(&b"Bar Baz"[..]));
}

#[test]
fn test_rss_parser_1() -> Result<(), TestParserError<'static>> {
    test_parser::<_, Step, _>(
        include_str!("./1.xml"),
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
            skip_hours: {
                let mut hours = SkipHours::default();
                (1..=3).for_each(|i| hours.set(i, true));

                hours
            },
            skip_days: SkipDays::new([0b0000_0111]),
        },
        [Entry {
            title: Some(Cow::Borrowed(b"entry 1")),
            link: Some(Cow::Borrowed(b"https://example.com/entry_1")),
            description: Some(Cow::Borrowed(b"the first entry")),
            ..Entry::default()
        }],
        &alloc::Dummy,
    )
}

#[test]
fn test_rss_parser_2() -> Result<(), TestParserError<'static>> {
    test_parser::<_, Step, _>(
        include_str!("./2.xml"),
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
