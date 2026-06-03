use {
    crate::{
        feed::{
            Entry, Feed, ParsedFeed, Parser, ParserError, PartialFeed, Period, decode_text_to_end,
        },
        utf8::{Event, Reader, Start},
    },
    jiff::{Span, Timestamp, civil::Weekday},
    std::{num::NonZeroU32, str::FromStr},
};

#[derive(Default)]
enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
    InsideSkipDays,
    InsideSkipHours,
}
#[derive(Default)]
pub struct RssParser<'a> {
    step: Step,
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
    fn output(self, before_send: Timestamp) -> Option<ParsedFeed<'a>> {
        Some(ParsedFeed {
            feed: Feed::from_partial(self.feed, before_send)?,
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
                    last_update: Timestamp::default(),
                },
                entries: vec![],
            },
            Timestamp::default(),
        )?;

        Ok(())
    }
}
