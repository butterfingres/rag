use {
    crate::feed::{Entry, Feed, ParsedFeed, Parser, ParserError, PartialFeed},
    chrono::{DateTime, FixedOffset},
    quick_xml::{
        events::{BytesStart, Event},
        reader::Reader,
    },
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
    fn from_start(tag: BytesStart) -> Result<Self, BytesStart> {
        tag.decoder()
            .decode(tag.local_name().into_inner())
            .ok()
            .filter(|tag| tag == "rss")
            .map(|_| Self::default())
            .ok_or(tag)
    }
    fn output(self, before_send: DateTime<FixedOffset>) -> Option<ParsedFeed> {
        Some(ParsedFeed {
            feed: Feed::from_partial(self.feed, before_send)?,
            entries: self.entries,
        })
    }
    fn handle_event(self, ev: Event<'_>, reader: &mut Reader<&[u8]>) -> Result<Self, ParserError> {
        match (self.step, ev) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name().0 == b"channel" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),
            (Step::InsideChannel, Event::End(tag)) if tag.name().0 == b"channel" => Ok(Self {
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
