use crate::{
    feed::{
        Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed, PartialText,
        decode_text_to_end,
    },
    utf8::{Event, Reader, Start},
};

#[derive(Default)]
enum Step<'a> {
    #[default]
    OutsideChannel,
    InsideChannel,
    InsideItems,
    InsideItem(PartialEntry<'a>),
}
#[derive(Default)]
pub struct Rss1Parser<'a> {
    step: Step<'a>,
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
    fn handle_event(mut self, ev: Event<'a>, reader: &mut Reader<'a>) -> Result<Self, ParserError> {
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

            (Step::OutsideChannel, Event::Start(tag)) if tag.local_name() == "item" => Ok(Self {
                step: Step::InsideItem(PartialEntry::default()),
                ..self
            }),
            (Step::InsideItem(item), Event::End(tag)) if tag.local_name() == "item" => {
                self.entries.push(item.into());
                Ok(Self {
                    step: Step::InsideItems,
                    ..self
                })
            }
            (Step::InsideItem(item), Event::Start(tag)) if tag.local_name() == "title" => {
                Ok(Self {
                    step: Step::InsideItem(PartialEntry {
                        title: Some(decode_text_to_end(reader, "title")?),
                        ..item
                    }),
                    ..self
                })
            }
            (Step::InsideItem(item), Event::Start(tag)) if tag.local_name() == "link" => Ok(Self {
                step: Step::InsideItem(PartialEntry {
                    link: Some(PartialText::strong(decode_text_to_end(reader, "link")?)),
                    ..item
                }),
                ..self
            }),
            (Step::InsideItem(item), Event::Start(tag)) if tag.local_name() == "description" => {
                Ok(Self {
                    step: Step::InsideItem(PartialEntry {
                        description: Some(PartialText::strong(decode_text_to_end(
                            reader,
                            "description",
                        )?)),
                        ..item
                    }),
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
                entries: vec![Entry {
                    title: Some(Cow::Borrowed("entry 1")),
                    link: Some(Cow::Borrowed("https://example.com/entry_1")),
                    description: Some(Cow::Borrowed("first entry")),
                    pub_date: None,
                    enclosures: vec![],
                }],
            },
        )?;

        Ok(())
    }
}
