use {
    super::*,
    crate::{
        tz,
        xml::tests::{TestParserError, test_parser},
    },
    jiff::civil::datetime,
    stumpalo::Arena,
};

#[test]
fn test_rss_parser() -> Result<(), TestParserError<'static>> {
    let mut arena = Arena::new();

    test_parser::<Step, _>(
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
                let mut hours = RssSkipHours::default();
                (1..=3).for_each(|i| hours.0.0.set(i, true));

                hours
            },
        },
        &arena,
    )?;
    arena.clear();

    test_parser::<Step, _>(
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
            skip_hours: RssSkipHours::default(),
        },
        &arena,
    )?;

    Ok(())
}
