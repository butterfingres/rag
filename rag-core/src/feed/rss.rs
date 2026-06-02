use {
    crate::{
        feed::{Entry, Feed, ParsedFeed, Parser, ParserError, PartialFeed},
        utf8::{Event, Reader, Start},
    },
    chrono::{DateTime, FixedOffset},
};

#[derive(Default)]
enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
}
#[derive(Default)]
pub struct RssParser {
    step: Step,
    feed: PartialFeed,
    entries: Vec<Entry>,
}
impl Parser for RssParser {
    fn from_start(tag: Start) -> Result<Self, Start> {
        if tag.local_name() == "rss" {
            Ok(Self::default())
        } else {
            Err(tag)
        }
    }
    fn output(self, before_send: DateTime<FixedOffset>) -> Option<ParsedFeed> {
        Some(ParsedFeed {
            feed: Feed::from_partial(self.feed, before_send)?,
            entries: self.entries,
        })
    }
    fn handle_event(self, ev: Event<'_>, reader: &mut Reader) -> Result<Self, ParserError> {
        match (self.step, ev) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),
            (Step::InsideChannel, Event::End(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::OutsideChannel,
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
