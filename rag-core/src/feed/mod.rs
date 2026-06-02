pub mod rss;

use {
    crate::utf8::{Event, Reader, Start},
    chrono::{DateTime, FixedOffset},
    quick_xml::{encoding::EncodingError, escape::resolve_xml_entity},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        str,
    },
};

pub enum Skip {
    Hour(u8),
    Weekday(u8),
}

pub enum UpdatePeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}
pub struct Update {
    pub period: UpdatePeriod,
    pub frequency: u32,
    pub base: DateTime<FixedOffset>,
}

#[derive(Default)]
pub struct PartialFeed {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: Option<DateTime<FixedOffset>>,
}
pub struct Feed {
    pub title: Box<str>,
    // The link is optional in atom.
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: DateTime<FixedOffset>,
}
impl Feed {
    pub fn from_partial(
        PartialFeed {
            title,
            link,
            skips,
            update,
            last_update,
        }: PartialFeed,
        before_send: DateTime<FixedOffset>,
    ) -> Option<Self> {
        Some(Self {
            title: title?,
            link,
            skips,
            update,
            last_update: last_update.unwrap_or(before_send),
        })
    }
}

pub struct PartialEntry {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub pub_date: Option<DateTime<FixedOffset>>,
    pub enclosures: Vec<Box<str>>,
}
pub struct Entry {
    pub title: Box<str>,
    pub link: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub pub_date: DateTime<FixedOffset>,
    pub enclosures: Vec<Box<str>>,
}

pub struct ParsedFeed {
    pub feed: Feed,
    pub entries: Vec<Entry>,
}

#[derive(Debug)]
pub enum ParserError {
    Encoding(EncodingError),
    Invalid,
    Xml(quick_xml::Error),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Encoding(e) => e.fmt(f),
            Self::Invalid => f.write_str("the feed does not conform to specifications"),
            Self::Xml(e) => e.fmt(f),
        }
    }
}
impl Error for ParserError {}
impl From<EncodingError> for ParserError {
    fn from(e: EncodingError) -> Self {
        Self::Encoding(e)
    }
}
impl From<quick_xml::Error> for ParserError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}
pub trait Parser
where
    Self: Sized,
{
    fn from_start(_: Start) -> Result<Self, Start>;
    fn output(self, _: DateTime<FixedOffset>) -> Option<ParsedFeed>;
    fn handle_event(self, _: Event<'_>, _: &mut Reader) -> Result<Self, ParserError>;

    fn parse(
        mut self,
        reader: &mut Reader,
        before_send: DateTime<FixedOffset>,
    ) -> Result<ParsedFeed, ParserError> {
        loop {
            match reader.read_event()? {
                Event::Eof => break self.output(before_send).ok_or(ParserError::Invalid),
                ev => {
                    self = self.handle_event(ev, reader)?;
                }
            }
        }
    }
}

pub fn decode_text_to_end(reader: &mut Reader, tag: &str) -> Result<Box<str>, ParserError> {
    let mut output = String::new();
    loop {
        match reader.read_event()? {
            Event::Text(text) => {
                output.push_str(text.as_ref());
            }
            Event::CData(data) => {
                output.push_str(data.as_ref());
            }
            Event::GeneralRef(ch) => {
                if let Some(ch) = ch.resolve_char_ref()? {
                    output.push(ch);
                } else if let Some(ch) = resolve_xml_entity(ch.as_ref_name()) {
                    output.push_str(ch);
                }
            }
            Event::Start(start) => reader.read_to_end(start.name()).map(|_| ())?,
            Event::End(end) if end.name() == tag => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(output.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_text_to_end() -> Result<(), ParserError> {
        assert_eq!(
            decode_text_to_end(&mut Reader::from_str("C &amp;lt; Rust"), "foo")?.as_ref(),
            "C &lt; Rust"
        );
        Ok(())
    }
}
