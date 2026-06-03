use {
    crate::{
        feed::{
            Authority, Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed,
            PartialText, Period, decode_text_to_end,
        },
        rfc822,
        utf8::{Event, Reader, Start},
    },
    jiff::{Span, civil::Weekday},
    std::{num::NonZeroU32, str::FromStr},
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
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "link" => Ok(Self {
                step,
                feed: PartialFeed {
                    link: Some(decode_text_to_end(reader, "link")?),
                    ..self.feed
                },
                ..self
            }),

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "pubDate" => {
                Ok(Self {
                    step,
                    feed: PartialFeed {
                        last_update: Some(rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?),
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
                    frequency: NonZeroU32::MIN,
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
                step: Step::InsideItem(PartialEntry::default().into()),
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
                    pub_date: Some(rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?),
                    ..entry
                }),
                ..self
            }),

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
        crate::feed::{Cache, SkipHours, SkipWeekdays},
        jiff::{
            civil::DateTime,
            tz::{TimeZone, offset},
        },
        std::borrow::Cow,
    };

    #[test]
    fn test_parser() -> Result<(), ParserError> {
        crate::feed::tests::test_parser::<RssParser>(
            "<rss>
  <channel>
    <title>hello world</title>
    <link>https://example.com</link>
    <skipDays>
      <day>Monday</day>
      <day>Tuesday</day>
      <day>Wednesday</day>
      <day>Thursday</day>
      <day>Friday</day>
      <day>Saturday</day>
      <day>Sunday</day>
    </skipDays>
    <skipHours>
      <hour>0</hour>
      <hour>23</hour>
    </skipHours>
    <ttl>69</ttl>
    <pubDate>Sat, 07 Sep 2002 00:00:01 GMT</pubDate>
    <item>
      <title>entry 1</title>
      <link>https://example.com</link>
      <link>https://example.com/foo</link>
      <pubDate>Fri, 21 Jul 2023 09:04 EDT</pubDate>
    </item>
  </channel>
</rss>",
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
                            frequency: NonZeroU32::MIN,
                            base: None,
                        }),
                    },
                    last_update: Some(
                        DateTime::new(2002, 09, 07, 00, 00, 01, 00)?
                            .to_zoned(TimeZone::fixed(offset(0)))?,
                    ),
                },
                entries: vec![Entry {
                    title: Some(Cow::Borrowed("entry 1")),
                    link: Some(Cow::Borrowed("https://example.com")),
                    description: None,
                    pub_date: Some(
                        DateTime::new(2023, 07, 21, 09, 04, 00, 00)?
                            .to_zoned(TimeZone::fixed(offset(-4)))?,
                    ),
                    enclosures: vec![],
                }],
            },
        )?;

        Ok(())
    }
}
