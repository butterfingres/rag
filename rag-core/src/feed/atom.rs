use {
    crate::{
        feed::{
            Authority, Entry, Feed, ParsedFeed, Parser, ParserError, PartialEntry, PartialFeed,
            PartialText, Period, decode_text_to_end,
        },
        utf8::{Attribute, Event, Reader, Start},
    },
    jiff::{
        Span, Timestamp,
        civil::{DateTime, Weekday},
    },
    std::{num::NonZeroU32, str::FromStr},
};

#[derive(Default)]
enum Step {
    #[default]
    InsideFeed,
}
#[derive(Default)]
pub struct AtomParser<'a> {
    step: Step,
    feed: PartialFeed<'a>,
    entries: Vec<Entry<'a>>,
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
        jiff::tz::TimeZone,
        std::borrow::Cow,
    };

    #[test]
    fn test_parser() -> Result<(), ParserError> {
        crate::feed::tests::test_parser::<AtomParser>(
            include_str!("./atom.xml"),
            ParsedFeed {
                feed: Feed {
                    title: Cow::Borrowed("example atom feed"),
                    link: None,
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
                entries: vec![],
            },
        )
    }
}
