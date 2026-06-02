use {
    crate::{
        feed::{Entry, Feed, ParsedFeed, Parser, ParserError, PartialFeed, decode_text_to_end},
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
    fn output(self, before_send: DateTime<FixedOffset>) -> Option<ParsedFeed<'a>> {
        Some(ParsedFeed {
            feed: Feed::from_partial(self.feed, before_send)?,
            entries: self.entries,
        })
    }
    fn handle_event(self, ev: Event<'a>, reader: &mut Reader<'a>) -> Result<Self, ParserError> {
        match (self.step, ev) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::InsideChannel,
                ..self
            }),
            (Step::InsideChannel, Event::End(tag)) if tag.name() == "channel" => Ok(Self {
                step: Step::OutsideChannel,
                ..self
            }),
            (Step::InsideChannel, Event::Start(tag)) if tag.name() == "title" => Ok(Self {
                step: Step::OutsideChannel,
                feed: PartialFeed {
                    title: Some(decode_text_to_end(reader, "title")?),
                    ..self.feed
                },
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
    use {super::*, std::borrow::Cow};

    #[test]
    fn test_parser() -> Result<(), ParserError> {
        crate::feed::tests::test_parser::<RssParser>(
            "<rss>
  <channel>
    <title>hello world</title>
  </channel>
</rss>",
            ParsedFeed {
                feed: Feed {
                    title: Cow::Borrowed("hello world"),
                    link: None,
                    skips: vec![],
                    update: None,
                    last_update: DateTime::default(),
                },
                entries: vec![],
            },
            DateTime::default(),
        )?;

        Ok(())
    }
}
