use {
    super::*,
    crate::{
        tz,
        xml::tests::{TestParserError, test_parser},
    },
    allocator_api2::alloc::Global,
    jiff::civil::datetime,
};

#[test]
fn test_parser_try_from_root() -> Result<(), TestParserError<'static>> {
    test_parser::<Step, _>(
        include_str!("./1.xml"),
        Channel {
            title: Some(Cow::Borrowed(b"example feed")),
            link: Some(Cow::Borrowed(b"https://example.com/rss")),
            modify_date: Replaceable {
                // Fri, 21 Jul 2023 09:04 EDT
                data: Some(
                    datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                ),
                replaceable: false,
            },
        },
        &Global,
    )
}
