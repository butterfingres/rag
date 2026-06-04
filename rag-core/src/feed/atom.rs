use {
    crate::{
        feed::{
            Authority, Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed,
            PartialText, decode_text_to_end,
        },
        utf8::{Event, Reader, Start},
    },
    jiff::Timestamp,
    std::{borrow::Cow, str::FromStr},
};

#[derive(Default)]
enum Step<'a> {
    #[default]
    InsideFeed,
    InsideEntry(PartialEntry<'a>),
}
#[derive(Default)]
pub struct AtomParser<'a> {
    step: Step<'a>,
    feed: PartialFeed<'a>,
    entries: Vec<Entry<'a>>,
}
impl<'a> AtomParser<'a> {
    fn handle_link(&mut self, tag: Start<'a>) -> Result<(), ParserError> {
        let mut href = None;
        let mut rel = None;
        for attr in tag.attributes() {
            let attr = attr?;
            match attr.key.local_name() {
                "href" => {
                    href = Some(attr.value);
                    if rel.is_some() {
                        break;
                    }
                }
                "rel" => {
                    if attr.value == "alternate" {
                        rel = Some(Authority::Strong);
                    } else {
                        rel = Some(Authority::Weak);
                        if href.is_some() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        let rel = rel.unwrap_or_default();
        if let Some(href) = href {
            PartialText::replace_text(
                &mut self.feed.link,
                PartialText {
                    text: Cow::Owned(href.into_owned()),
                    authority: rel,
                },
            );
        }

        Ok(())
    }
}
impl<'a> Parser<'a> for AtomParser<'a> {
    fn try_from_root(tag: Start) -> Result<Self, Start> {
        if tag.local_name() == "feed" {
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
            (step @ Step::InsideFeed, Event::Start(tag)) if tag.name() == "title" => Ok(Self {
                step,
                feed: PartialFeed {
                    title: Some(decode_text_to_end(reader, "title")?),
                    ..self.feed
                },
                ..self
            }),
            (step @ Step::InsideFeed, Event::Start(tag)) if tag.name() == "updated" => Ok(Self {
                step,
                feed: PartialFeed {
                    last_update: Some(Timestamp::from_str(&decode_text_to_end(
                        reader, "updated",
                    )?)?),
                    ..self.feed
                },
                ..self
            }),
            (step @ Step::InsideFeed, Event::Start(tag)) if tag.name() == "link" => {
                reader.read_to_end("link")?;
                self = Self { step, ..self };
                self.handle_link(tag)?;
                Ok(self)
            }
            (step @ Step::InsideFeed, Event::Empty(tag)) if tag.name() == "link" => {
                self = Self { step, ..self };
                self.handle_link(tag)?;
                Ok(self)
            }

            (Step::InsideFeed, Event::Start(tag)) if tag.name() == "entry" => Ok(Self {
                step: Step::InsideEntry(PartialEntry::default()),
                ..self
            }),
            (Step::InsideEntry(entry), Event::End(tag)) if tag.name() == "entry" => {
                self.entries.push(entry.into());
                Ok(Self {
                    step: Step::InsideFeed,
                    ..self
                })
            }
            (Step::InsideEntry(mut entry), Event::Start(tag)) if tag.name() == "title" => {
                entry.title = Some(decode_text_to_end(reader, "title")?);

                Ok(Self {
                    step: Step::InsideEntry(entry),
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
        jiff::{civil::DateTime, tz::TimeZone},
        std::borrow::Cow,
    };

    #[test]
    fn test_parser() -> Result<(), ParserError> {
        crate::feed::tests::test_parser::<AtomParser>(
            include_str!("./atom.xml"),
            ParsedFeed {
                feed: Feed {
                    title: Cow::Borrowed("example atom feed"),
                    link: Some(Cow::Borrowed("https://example.com/")),
                    // we need to test that all the weekdays are recognized
                    cache: Cache {
                        skip_weekdays: SkipWeekdays::default(),
                        skip_hours: SkipHours::default(),
                        period: None,
                    },
                    last_update: Some(
                        DateTime::new(2003, 12, 13, 18, 30, 02, 00)?
                            .to_zoned(TimeZone::fixed(tz::Z))?
                            .timestamp(),
                    ),
                },
                entries: vec![Entry {
                    title: Some(Cow::Borrowed("entry 1")),
                    link: None,
                    description: None,
                    pub_date: None,
                    enclosures: vec![],
                }],
            },
        )
    }
}
