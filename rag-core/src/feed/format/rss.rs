use {
    crate::{
        feed::{
            Authority, Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed,
            PartialText, Period, decode_text_to_end,
        },
        rfc822,
        utf8::{Attribute, Event, Reader, Start},
    },
    jiff::{Span, civil::Weekday},
    std::{borrow::Cow, num::NonZeroU16, str::FromStr},
};

#[derive(Default)]
enum Step<'a> {
    #[default]
    OutsideChannel,
    InsideChannel,
    InsideSkipDays,
    InsideSkipHours,
    InsideItem(PartialEntry<'a>),
}
#[derive(Default)]
pub struct RssParser<'a> {
    step: Step<'a>,
    feed: PartialFeed<'a>,
    entries: Vec<Entry<'a>>,
}
impl<'a> Parser<'a> for RssParser<'a> {
    fn try_from_root(tag: Start) -> Result<Self, Start> {
        if tag.local_name() == "rss" {
            Ok(Self::default())
        } else {
            Err(tag)
        }
    }
    fn output(self) -> Option<ParsedFeed<'a>> {
        Some(ParsedFeed {
            feed: Feed::from_partial(self.feed)?,
            entries: self.entries,
        })
    }
    // TODO: use borrowed attributes
    fn handle_event(mut self, ev: Event<'a>, reader: &mut Reader<'a>) -> Result<Self, ParserError> {
        match (self.step, ev) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),
            (Step::InsideChannel, Event::End(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::OutsideChannel,
                ..self
            }),
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "title" => Ok(Self {
                step,
                feed: PartialFeed {
                    title: Some(decode_text_to_end(reader, "title")?),
                    ..self.feed
                },
                ..self
            }),
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "link" => {
                PartialText::replace_with_text_or_skip(
                    &mut self.feed.link,
                    "link",
                    reader,
                    Authority::Strong,
                )?;
                Ok(Self { step, ..self })
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "pubDate" => {
                Ok(Self {
                    step,
                    feed: PartialFeed {
                        last_update: Some(
                            rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?.timestamp(),
                        ),
                        ..self.feed
                    },
                    ..self
                })
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "ttl" => {
                let mins = decode_text_to_end(reader, "ttl")?;
                let mins = i64::from_str(&mins)?;
                self.feed.cache.period = Some(Period {
                    interval: Span::new().try_minutes(mins)?.into(),
                    base: None,
                    frequency: NonZeroU16::MIN,
                });

                Ok(Self { step, ..self })
            }

            (Step::InsideChannel, Event::Start(tag)) if tag.name() == "skipDays" => Ok(Self {
                step: Step::InsideSkipDays,
                ..self
            }),
            (step @ Step::InsideSkipDays, Event::Start(tag)) if tag.name() == "day" => {
                let day = decode_text_to_end(reader, "day")?;
                let day = match day.as_ref() {
                    "Monday" => Ok(Weekday::Monday),
                    "Tuesday" => Ok(Weekday::Tuesday),
                    "Wednesday" => Ok(Weekday::Wednesday),
                    "Thursday" => Ok(Weekday::Thursday),
                    "Friday" => Ok(Weekday::Friday),
                    "Saturday" => Ok(Weekday::Saturday),
                    "Sunday" => Ok(Weekday::Sunday),
                    _ => Err(ParserError::ParseWeekday(Box::from(day))),
                }?;
                self.feed.cache.skip_weekdays.set(
                    usize::try_from(day.to_monday_zero_offset()).expect(
                        "[Weekday] is `repr(u8)` meaning it would always fit in an [usize]",
                    ),
                    true,
                );

                Ok(Self { step, ..self })
            }
            (Step::InsideSkipDays, Event::End(tag)) if tag.name() == "skipDays" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),

            (Step::InsideChannel, Event::Start(tag)) if tag.name() == "skipHours" => Ok(Self {
                step: Step::InsideSkipHours,
                ..self
            }),
            (step @ Step::InsideSkipHours, Event::Start(tag)) if tag.name() == "hour" => {
                let hour = decode_text_to_end(reader, "hour")?;
                let hour = usize::from_str(&hour)?;
                self.feed.cache.skip_hours.set(hour, true);

                Ok(Self { step, ..self })
            }
            (Step::InsideSkipHours, Event::End(tag)) if tag.name() == "skipHours" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),

            (Step::InsideChannel, Event::Start(tag)) if tag.name() == "item" => Ok(Self {
                step: Step::InsideItem(PartialEntry::default()),
                ..self
            }),
            (Step::InsideItem(entry), Event::End(tag)) if tag.name() == "item" => {
                self.entries.push(entry.into());
                Ok(Self {
                    step: Step::InsideChannel,
                    ..self
                })
            }
            (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "title" => {
                entry.title = Some(decode_text_to_end(reader, "title")?);
                Ok(Self {
                    step: Step::InsideItem(entry),
                    ..self
                })
            }
            (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "link" => {
                PartialText::replace_with_text_or_skip(
                    &mut entry.link,
                    "link",
                    reader,
                    Authority::Strong,
                )?;
                Ok(Self {
                    step: Step::InsideItem(entry),
                    ..self
                })
            }
            (Step::InsideItem(entry), Event::Start(tag)) if tag.name() == "pubDate" => Ok(Self {
                step: Step::InsideItem(PartialEntry {
                    pub_date: Some(
                        rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?.timestamp(),
                    ),
                    ..entry
                }),
                ..self
            }),
            (Step::InsideItem(entry), Event::Start(tag)) if tag.name() == "description" => {
                Ok(Self {
                    step: Step::InsideItem(PartialEntry {
                        description: Some(PartialText {
                            text: decode_text_to_end(reader, "description")?,
                            authority: Authority::Strong,
                        }),
                        ..entry
                    }),
                    ..self
                })
            }
            (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "enclosure" => {
                reader.read_to_end("enclosure")?;
                if let Some(Attribute { value, .. }) = tag.try_get_attribute("url")? {
                    entry.enclosures.push(Cow::Owned(value.into_owned()));
                }

                Ok(Self {
                    step: Step::InsideItem(entry),
                    ..self
                })
            }
            (Step::InsideItem(mut entry), Event::Empty(tag)) if tag.name() == "enclosure" => {
                if let Some(Attribute { value, .. }) = tag.try_get_attribute("url")? {
                    entry.enclosures.push(Cow::Owned(value.into_owned()));
                }

                Ok(Self {
                    step: Step::InsideItem(entry),
                    ..self
                })
            }
            (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "guid" => {
                if tag
                    .try_get_attribute("isPermaLink")?
                    .is_none_or(|Attribute { value, .. }| value != "false")
                {
                    PartialText::replace_with_text_or_skip(
                        &mut entry.link,
                        "guid",
                        reader,
                        Authority::Strong,
                    )?;
                } else {
                    reader.read_to_end("guid")?;
                }
                Ok(Self {
                    step: Step::InsideItem(entry),
                    ..self
                })
            }

            (step, Event::Start(tag)) => {
                reader.read_to_end(tag.name())?;
                Ok(Self { step, ..self })
            }
            (step, _) => Ok(Self { step, ..self }),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            feed::{Cache, SkipHours, SkipWeekdays},
            tz,
        },
        jiff::{
            civil::DateTime,
            tz::{TimeZone, offset},
        },
        std::borrow::Cow,
    };

    #[test]
    fn test_parser() -> Result<(), ParserError> {
        crate::feed::tests::test_parser::<RssParser>(
            include_str!("./rss.xml"),
            ParsedFeed {
                feed: Feed {
                    title: Cow::Borrowed("hello world"),
                    link: Some(Cow::Borrowed("https://example.com")),
                    // we need to test that all the weekdays are recognized
                    cache: Cache {
                        skip_weekdays: SkipWeekdays::new([0b0111_1111]),
                        skip_hours: SkipHours::new([0b0000_0000_1000_0000_0000_0000_0000_0001]),
                        period: Some(Period {
                            interval: Span::new().try_minutes(69)?.into(),
                            frequency: NonZeroU16::MIN,
                            base: None,
                        }),
                    },
                    last_update: Some(
                        DateTime::new(2002, 09, 07, 00, 00, 01, 00)?
                            .to_zoned(TimeZone::fixed(offset(0)))?
                            .timestamp(),
                    ),
                },
                entries: vec![
                    Entry {
                        title: Some(Cow::Borrowed("entry 1")),
                        link: Some(Cow::Borrowed("https://example.com")),
                        description: Some(Cow::Borrowed("example rss entry description")),
                        pub_date: Some(
                            DateTime::new(2023, 07, 21, 09, 04, 00, 00)?
                                .to_zoned(TimeZone::fixed(tz::EDT))?
                                .timestamp(),
                        ),
                        enclosures: vec![
                            Cow::Borrowed("https://example.com/audio.mp3"),
                            Cow::Borrowed("https://example.com/video.mp4"),
                        ],
                    },
                    Entry {
                        title: None,
                        link: Some(Cow::Borrowed("https://example.com/entry_2")),
                        description: None,
                        pub_date: None,
                        enclosures: vec![],
                    },
                    Entry {
                        title: None,
                        link: None,
                        description: None,
                        pub_date: None,
                        enclosures: vec![],
                    },
                    Entry {
                        title: None,
                        link: Some(Cow::Borrowed("https://example.com/entry_3")),
                        description: None,
                        pub_date: None,
                        enclosures: vec![],
                    },
                ],
            },
        )?;

        Ok(())
    }
}
