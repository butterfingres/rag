pub mod atom;
pub mod rdf;
pub mod rss;

#[cfg(test)]
mod tests {
    use {
        crate::{
            alloc::with_bump,
            borrow::Cow,
            xml::{
                Entry, Feed, Parser, SkipDays, SkipHours,
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::{boxed::Box, vec},
        bump_scope::Bump,
        jiff::{Span, civil::datetime, tz::TimeZone},
    };

    pub fn test_parser_ns<'src, T>(input: &'src str) -> Result<(), TestParserError<'src>>
    where
        T: for<'alloc> Parser<'alloc, 'src, Bump>,
    {
        with_bump(|alloc| {
            test_parser::<_, T, Bump>(
                input,
                Feed {
                    title: Some(Cow::Borrowed(b"dc title")),
                    link: None,
                    // 2026-07-03
                    last_update: Some(
                        datetime(2026, 07, 03, 00, 00, 00, 00)
                            .to_zoned(TimeZone::UTC)?
                            .timestamp(),
                    ),
                    skip_hours: SkipHours::default(),
                    skip_days: SkipDays::default(),
                    ttl: Span::new().try_hours(1)?,
                    frequency: Some(2),
                },
                [
                    Entry {
                        title: Some(Cow::Borrowed(b"dublin core entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"dublin core entry description")),
                        id: Some(Cow::Borrowed(b"1")),
                        // 2026-07-03
                        pub_date: Some(
                            datetime(2026, 07, 03, 00, 00, 00, 00)
                                .to_zoned(TimeZone::UTC)?
                                .timestamp(),
                        ),
                        enclosures: vec![in &alloc;],
                    },
                    Entry {
                        title: None,
                        link: None,
                        description: Some(Cow::Borrowed(b"content description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"media entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"media description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;
                                         Box::slice(Box::new_in(*b"https://example.com/media.mp3", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media.mp4", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media.torrent", &alloc)),
                        ],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"media group entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"media group description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.mp3", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.mp4", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.torrent", &alloc)),
                        ],
                    },
                ],
                &alloc,
            )
        })
    }
}
