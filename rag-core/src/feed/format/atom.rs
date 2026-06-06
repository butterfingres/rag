use {
    crate::{
        feed::format::{
            Authority, Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed,
            PartialText, decode_text_to_end,
        },
        utf8::{Event, Reader, Start},
    },
    jiff::Timestamp,
    std::{borrow::Cow, str::FromStr},
};

#[derive(Default)]
enum Rel {
    Alternate,
    Enclosure,
    #[default]
    Other,
}
impl From<Rel> for Authority {
    fn from(rel: Rel) -> Self {
        match rel {
            Rel::Alternate => Authority::Strong,
            Rel::Enclosure | Rel::Other => Authority::Weak,
        }
    }
}

struct Link<'a> {
    uri: Cow<'a, str>,
    rel: Rel,
}
impl<'a> From<Link<'a>> for PartialText<'a> {
    fn from(Link { uri, rel }: Link) -> PartialText {
        PartialText {
            text: uri,
            authority: rel.into(),
        }
    }
}

trait HandleLink<'a> {
    fn handle_link(&mut self, _link: Link<'a>);

    fn handle_link_tag(&mut self, tag: Start<'a>) -> Result<(), ParserError> {
        let mut href = None;
        let mut rel = None;
        for attr in tag.attributes() {
            let attr = attr?;
            match attr.key.local_name() {
                "href" => {
                    href = Some(Cow::Owned(attr.value.into_owned()));
                    if rel.is_some() {
                        break;
                    }
                }
                "rel" => {
                    rel = Some(match attr.value.as_ref() {
                        "alternate" => Rel::Alternate,
                        "enclosure" => Rel::Enclosure,
                        _ => Rel::Other,
                    });
                    if href.is_some() {
                        break;
                    }
                }
                _ => {}
            }
        }

        let rel = rel.unwrap_or_default();
        if let Some(uri) = href {
            self.handle_link(Link { rel, uri });
        }

        Ok(())
    }
}
impl<'a> HandleLink<'a> for AtomParser<'a> {
    fn handle_link(&mut self, link: Link<'a>) {
        PartialText::replace_text(&mut self.feed.link, link.into());
    }
}
impl<'a> HandleLink<'a> for PartialEntry<'a> {
    fn handle_link(&mut self, link: Link<'a>) {
        if let Rel::Enclosure = link.rel {
            self.enclosures.push(link.uri);
        } else {
            PartialText::replace_text(&mut self.link, link.into());
        }
    }
}

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
impl<'a> AtomParser<'a> {}
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
                self.handle_link_tag(tag)?;
                Ok(self)
            }
            (step @ Step::InsideFeed, Event::Empty(tag)) if tag.name() == "link" => {
                self = Self { step, ..self };
                self.handle_link_tag(tag)?;
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
            (Step::InsideEntry(mut entry), Event::Start(tag)) if tag.name() == "link" => {
                reader.read_to_end("link")?;
                entry.handle_link_tag(tag)?;
                Ok(Self {
                    step: Step::InsideEntry(entry),
                    ..self
                })
            }
            (Step::InsideEntry(mut entry), Event::Empty(tag)) if tag.name() == "link" => {
                entry.handle_link_tag(tag)?;
                Ok(Self {
                    step: Step::InsideEntry(entry),
                    ..self
                })
            }
            (Step::InsideEntry(entry), Event::Start(tag)) if tag.name() == "updated" => Ok(Self {
                step: Step::InsideEntry(PartialEntry {
                    pub_date: Some(Timestamp::from_str(&decode_text_to_end(
                        reader, "updated",
                    )?)?),
                    ..entry
                }),
                ..self
            }),
            (Step::InsideEntry(mut entry), Event::Start(tag)) if tag.name() == "content" => {
                PartialText::replace_with_text_or_skip(
                    &mut entry.description,
                    "content",
                    reader,
                    Authority::Strong,
                )?;
                Ok(Self {
                    step: Step::InsideEntry(entry),
                    ..self
                })
            }
            (Step::InsideEntry(mut entry), Event::Start(tag)) if tag.name() == "summary" => {
                PartialText::replace_with_text_or_skip(
                    &mut entry.description,
                    "summary",
                    reader,
                    Authority::Weak,
                )?;
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
            feed::format::{Cache, SkipHours, SkipWeekdays},
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
        crate::feed::format::tests::test_parser::<AtomParser>(
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
                    link: Some(Cow::Borrowed("https://example.com/entry_1")),
                    description: Some(Cow::Borrowed("first post content")),
                    pub_date: Some(
                        DateTime::new(2003, 12, 13, 18, 30, 02, 00)?
                            .to_zoned(TimeZone::fixed(offset(-5)))?
                            .timestamp(),
                    ),
                    enclosures: vec![
                        Cow::Borrowed("https://example.com/audio_enclosure.mp3"),
                        Cow::Borrowed("https://example.com/video_enclosure.mp4"),
                    ],
                }],
            },
        )
    }
}
