pub mod rss;

use {
    chrono::{DateTime, FixedOffset},
    quick_xml::{
        encoding::EncodingError,
        escape::resolve_xml_entity,
        events::{BytesStart, Event},
        name::QName,
        reader::Reader,
    },
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
    fn from_start(_: BytesStart) -> Result<Self, BytesStart>;
    fn output(self, _: DateTime<FixedOffset>) -> Option<ParsedFeed>;
    /// # Safety
    ///
    /// The [Reader] must only have utf-8 data.
    unsafe fn handle_event(self, _: Event<'_>, _: &mut Reader<&[u8]>) -> Result<Self, ParserError>;

    /// # Safety
    ///
    /// See [Self::handle_event].
    unsafe fn parse(
        mut self,
        reader: &mut Reader<&[u8]>,
        before_send: DateTime<FixedOffset>,
    ) -> Result<ParsedFeed, ParserError> {
        loop {
            match reader.read_event()? {
                Event::Eof => break self.output(before_send).ok_or(ParserError::Invalid),
                ev => {
                    self = unsafe { self.handle_event(ev, reader) }?;
                }
            }
        }
    }
}

/// # Safety
///
/// The [Reader] must always be utf-8.
pub unsafe fn decode_text_to_end(
    reader: &mut Reader<&[u8]>,
    tag: QName<'_>,
) -> Result<Box<str>, ParserError> {
    let mut output = String::new();
    loop {
        match reader.read_event()? {
            Event::Text(text) => {
                // SAFETY: `reader` must be utf-8
                output.push_str(unsafe { str::from_utf8_unchecked(text.as_ref()) });
            }
            Event::CData(data) => {
                // SAFETY: `reader` must be utf-8
                output.push_str(unsafe { str::from_utf8_unchecked(data.as_ref()) });
            }
            Event::GeneralRef(ch) => {
                if let Some(ch) = ch.resolve_char_ref()? {
                    output.push(ch);
                } else if let Some(ch) =
                    // SAFETY: `reader` must be utf-8
                    resolve_xml_entity(unsafe {
                        str::from_utf8_unchecked(ch.as_ref())
                    })
                {
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

    fn decode_text_to_end_safe(text: &str, tag: QName) -> Result<Box<str>, ParserError> {
        // SAFETY: `text` is a utf-8 [str]
        unsafe { decode_text_to_end(&mut Reader::from_str(text), tag) }
    }

    #[test]
    fn test_decode_text_to_end() -> Result<(), ParserError> {
        assert_eq!(
            decode_text_to_end_safe("C &amp;lt; Rust", QName(b"foo"))?.as_ref(),
            "C &lt; Rust"
        );
        Ok(())
    }
}
