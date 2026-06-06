use crate::{
    feed::{
        Entry, Feed, ParsedFeed, Parser, ParserError, PartialFeed, PartialText, decode_text_to_end,
    },
    utf8::{Event, Reader, Start},
};

#[derive(Default)]
enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
}
#[derive(Default)]
pub struct Rss1Parser<'a> {
    step: Step,
    feed: PartialFeed<'a>,
    entries: Vec<Entry<'a>>,
}
impl<'a> Parser<'a> for Rss1Parser<'a> {
    fn try_from_root(tag: Start) -> Result<Self, Start> {
        if tag.local_name() == "RDF" {
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
    fn handle_event(self, ev: Event<'a>, reader: &mut Reader<'a>) -> Result<Self, ParserError> {
        match (self.step, ev) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.local_name() == "channel" => {
                Ok(Self {
                    step: Step::InsideChannel,
                    ..self
                })
            }
            (Step::InsideChannel, Event::End(tag)) if tag.local_name() == "channel" => Ok(Self {
                step: Step::OutsideChannel,
                ..self
            }),

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.local_name() == "title" => {
                Ok(Self {
                    step,
                    feed: PartialFeed {
                        title: Some(decode_text_to_end(reader, "title")?),
                        ..self.feed
                    },
                    ..self
                })
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.local_name() == "link" => {
                Ok(Self {
                    step,
                    feed: PartialFeed {
                        link: Some(PartialText::strong(decode_text_to_end(reader, "link")?)),
                        ..self.feed
                    },
                    ..self
                })
            }
            // (Step::OutsideChannel, Event::Start(tag)) if tag.name() == "channel" => Ok(Self {
            //     step: Step::InsideChannel,
            //     ..self
            // }),
            // (Step::InsideChannel, Event::End(tag)) if tag.name() == "channel" => Ok(Self {
            //     step: Step::OutsideChannel,
            //     ..self
            // }),
            // (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "title" => Ok(Self {
            //     step,
            //     feed: PartialFeed {
            //         title: Some(decode_text_to_end(reader, "title")?),
            //         ..self.feed
            //     },
            //     ..self
            // }),
            // (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "link" => {
            //     PartialText::replace_with_text_or_skip(
            //         &mut self.feed.link,
            //         "link",
            //         reader,
            //         Authority::Strong,
            //     )?;
            //     Ok(Self { step, ..self })
            // }

            // (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "pubDate" => {
            //     Ok(Self {
            //         step,
            //         feed: PartialFeed {
            //             last_update: Some(
            //                 rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?.timestamp(),
            //             ),
            //             ..self.feed
            //         },
            //         ..self
            //     })
            // }
            // (step @ Step::InsideChannel, Event::Start(tag)) if tag.name() == "ttl" => {
            //     let mins = decode_text_to_end(reader, "ttl")?;
            //     let mins = i64::from_str(&mins)?;
            //     self.feed.cache.period = Some(Period {
            //         interval: Span::new().try_minutes(mins)?.into(),
            //         base: None,
            //         frequency: NonZeroU32::MIN,
            //     });

            //     Ok(Self { step, ..self })
            // }

            // (Step::InsideChannel, Event::Start(tag)) if tag.name() == "skipDays" => Ok(Self {
            //     step: Step::InsideSkipDays,
            //     ..self
            // }),
            // (step @ Step::InsideSkipDays, Event::Start(tag)) if tag.name() == "day" => {
            //     let day = decode_text_to_end(reader, "day")?;
            //     let day = match day.as_ref() {
            //         "Monday" => Ok(Weekday::Monday),
            //         "Tuesday" => Ok(Weekday::Tuesday),
            //         "Wednesday" => Ok(Weekday::Wednesday),
            //         "Thursday" => Ok(Weekday::Thursday),
            //         "Friday" => Ok(Weekday::Friday),
            //         "Saturday" => Ok(Weekday::Saturday),
            //         "Sunday" => Ok(Weekday::Sunday),
            //         _ => Err(ParserError::ParseWeekday(Box::from(day))),
            //     }?;
            //     self.feed.cache.skip_weekdays.set(
            //         usize::try_from(day.to_monday_zero_offset()).expect(
            //             "[Weekday] is `repr(u8)` meaning it would always fit in an [usize]",
            //         ),
            //         true,
            //     );

            //     Ok(Self { step, ..self })
            // }
            // (Step::InsideSkipDays, Event::End(tag)) if tag.name() == "skipDays" => Ok(Self {
            //     step: Step::InsideChannel,
            //     ..self
            // }),

            // (Step::InsideChannel, Event::Start(tag)) if tag.name() == "skipHours" => Ok(Self {
            //     step: Step::InsideSkipHours,
            //     ..self
            // }),
            // (step @ Step::InsideSkipHours, Event::Start(tag)) if tag.name() == "hour" => {
            //     let hour = decode_text_to_end(reader, "hour")?;
            //     let hour = usize::from_str(&hour)?;
            //     self.feed.cache.skip_hours.set(hour, true);

            //     Ok(Self { step, ..self })
            // }
            // (Step::InsideSkipHours, Event::End(tag)) if tag.name() == "skipHours" => Ok(Self {
            //     step: Step::InsideChannel,
            //     ..self
            // }),

            // (Step::InsideChannel, Event::Start(tag)) if tag.name() == "item" => Ok(Self {
            //     step: Step::InsideItem(PartialEntry::default()),
            //     ..self
            // }),
            // (Step::InsideItem(entry), Event::End(tag)) if tag.name() == "item" => {
            //     self.entries.push(entry.into());
            //     Ok(Self {
            //         step: Step::InsideChannel,
            //         ..self
            //     })
            // }
            // (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "title" => {
            //     entry.title = Some(decode_text_to_end(reader, "title")?);
            //     Ok(Self {
            //         step: Step::InsideItem(entry),
            //         ..self
            //     })
            // }
            // (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "link" => {
            //     PartialText::replace_with_text_or_skip(
            //         &mut entry.link,
            //         "link",
            //         reader,
            //         Authority::Strong,
            //     )?;
            //     Ok(Self {
            //         step: Step::InsideItem(entry),
            //         ..self
            //     })
            // }
            // (Step::InsideItem(entry), Event::Start(tag)) if tag.name() == "pubDate" => Ok(Self {
            //     step: Step::InsideItem(PartialEntry {
            //         pub_date: Some(
            //             rfc822::parse(&decode_text_to_end(reader, "pubDate")?)?.timestamp(),
            //         ),
            //         ..entry
            //     }),
            //     ..self
            // }),
            // (Step::InsideItem(entry), Event::Start(tag)) if tag.name() == "description" => {
            //     Ok(Self {
            //         step: Step::InsideItem(PartialEntry {
            //             description: Some(PartialText {
            //                 text: decode_text_to_end(reader, "description")?,
            //                 authority: Authority::Strong,
            //             }),
            //             ..entry
            //         }),
            //         ..self
            //     })
            // }
            // (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "enclosure" => {
            //     reader.read_to_end("enclosure")?;
            //     if let Some(Attribute { value, .. }) = tag.try_get_attribute("url")? {
            //         entry.enclosures.push(Cow::Owned(value.into_owned()));
            //     }

            //     Ok(Self {
            //         step: Step::InsideItem(entry),
            //         ..self
            //     })
            // }
            // (Step::InsideItem(mut entry), Event::Empty(tag)) if tag.name() == "enclosure" => {
            //     if let Some(Attribute { value, .. }) = tag.try_get_attribute("url")? {
            //         entry.enclosures.push(Cow::Owned(value.into_owned()));
            //     }

            //     Ok(Self {
            //         step: Step::InsideItem(entry),
            //         ..self
            //     })
            // }
            // (Step::InsideItem(mut entry), Event::Start(tag)) if tag.name() == "guid" => {
            //     if tag
            //         .try_get_attribute("isPermaLink")?
            //         .is_none_or(|Attribute { value, .. }| value != "false")
            //     {
            //         PartialText::replace_with_text_or_skip(
            //             &mut entry.link,
            //             "guid",
            //             reader,
            //             Authority::Strong,
            //         )?;
            //     } else {
            //         reader.read_to_end("guid")?;
            //     }
            //     Ok(Self {
            //         step: Step::InsideItem(entry),
            //         ..self
            //     })
            // }
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
        crate::feed::tests::test_parser::<Rss1Parser>(
            include_str!("./rss_1_0.xml"),
            ParsedFeed {
                feed: Feed {
                    title: Cow::Borrowed("rss 1.0 feed"),
                    link: Some(Cow::Borrowed("https://example.com")),
                    // we need to test that all the weekdays are recognized
                    cache: Cache {
                        skip_weekdays: SkipWeekdays::default(),
                        skip_hours: SkipHours::default(),
                        period: None,
                    },
                    last_update: None,
                },
                entries: vec![],
            },
        )?;

        Ok(())
    }
}
