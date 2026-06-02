pub mod rss;

use {
    crate::utf8::{Event, Reader, Start},
    chrono::{DateTime, FixedOffset, ParseWeekdayError, Weekday},
    quick_xml::{encoding::EncodingError, escape::resolve_xml_entity},
    std::{
        borrow::Cow,
        error::Error,
        fmt::{self, Display, Formatter},
        num::{ParseIntError, TryFromIntError},
        str,
    },
};

#[derive(Debug, PartialEq)]
pub enum Skip {
    Hour(u8),
    Weekday(Weekday),
}
#[derive(Debug, PartialEq)]
pub enum UpdatePeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}
#[derive(Debug, PartialEq)]
pub struct Update {
    pub period: UpdatePeriod,
    pub frequency: u32,
    pub base: DateTime<FixedOffset>,
}

#[derive(Default)]
pub struct PartialFeed<'a> {
    pub title: Option<Cow<'a, str>>,
    pub link: Option<Cow<'a, str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: Option<DateTime<FixedOffset>>,
}
#[derive(Debug, PartialEq)]
pub struct Feed<'a> {
    pub title: Cow<'a, str>,
    // The link is optional in atom.
    pub link: Option<Cow<'a, str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: DateTime<FixedOffset>,
}
impl<'a> Feed<'a> {
    pub fn from_partial(
        PartialFeed {
            title,
            link,
            skips,
            update,
            last_update,
        }: PartialFeed<'a>,
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

pub struct PartialEntry<'a> {
    pub title: Option<Cow<'a, str>>,
    pub link: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub pub_date: Option<DateTime<FixedOffset>>,
    pub enclosures: Vec<Cow<'a, str>>,
}
#[derive(Debug, PartialEq)]
pub struct Entry<'a> {
    pub title: Cow<'a, str>,
    pub link: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub pub_date: DateTime<FixedOffset>,
    pub enclosures: Vec<Cow<'a, str>>,
}

#[derive(Debug, PartialEq)]
pub struct ParsedFeed<'a> {
    pub feed: Feed<'a>,
    pub entries: Vec<Entry<'a>>,
}

#[derive(Debug)]
pub enum ParserError {
    Encoding(EncodingError),
    Invalid,
    ParseInt(ParseIntError),
    ParseWeekday(ParseWeekdayError),
    Xml(quick_xml::Error),
    TryFromInt(TryFromIntError),
    UnrecognizedRoot(Option<Box<str>>),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Encoding(e) => e.fmt(f),
            Self::Invalid => f.write_str("the feed does not conform to specifications"),
            Self::ParseInt(e) => e.fmt(f),
            Self::ParseWeekday(e) => e.fmt(f),
            Self::Xml(e) => e.fmt(f),
            Self::TryFromInt(e) => e.fmt(f),
            Self::UnrecognizedRoot(Some(tag)) => write!(f, "unrecognized root element `{tag}`"),
            Self::UnrecognizedRoot(None) => f.write_str("root element is not an element"),
        }
    }
}
impl Error for ParserError {}
impl From<EncodingError> for ParserError {
    fn from(e: EncodingError) -> Self {
        Self::Encoding(e)
    }
}
impl From<ParseIntError> for ParserError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
impl From<ParseWeekdayError> for ParserError {
    fn from(e: ParseWeekdayError) -> Self {
        Self::ParseWeekday(e)
    }
}
impl From<quick_xml::Error> for ParserError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}
impl From<TryFromIntError> for ParserError {
    fn from(e: TryFromIntError) -> Self {
        Self::TryFromInt(e)
    }
}

pub trait Parser<'a>
where
    Self: Sized,
{
    fn try_from_root(_: Start) -> Result<Self, Start>;
    fn output(self, _: DateTime<FixedOffset>) -> Option<ParsedFeed<'a>>;
    fn handle_event(self, _: Event<'a>, _: &mut Reader<'a>) -> Result<Self, ParserError>;

    fn parse(
        mut self,
        reader: &mut Reader<'a>,
        before_send: DateTime<FixedOffset>,
    ) -> Result<ParsedFeed<'a>, ParserError> {
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

pub fn decode_text_to_end<'a>(
    reader: &mut Reader<'a>,
    tag: &str,
) -> Result<Cow<'a, str>, ParserError> {
    let mut output = Cow::Borrowed("");
    let start = usize::try_from(reader.buffer_position())?;
    let slice = reader.as_str();

    loop {
        match reader.read_event()? {
            Event::Text(text) => match output {
                Cow::Borrowed(_) => {
                    output =
                        Cow::Borrowed(&slice[..usize::try_from(reader.buffer_position())? - start]);
                }
                Cow::Owned(_) => {
                    output.to_mut().push_str(text.as_ref());
                }
            },
            Event::CData(data) => {
                output.to_mut().push_str(data.as_ref());
            }
            Event::GeneralRef(ch) => {
                if let Some(ch) = ch.resolve_char_ref()? {
                    output.to_mut().push(ch);
                } else if let Some(ch) = resolve_xml_entity(ch.as_ref_name()) {
                    output.to_mut().push_str(ch);
                }
            }
            Event::Start(start) => reader.read_to_end(start.name()).map(|_| ())?,
            Event::End(end) if end.name() == tag => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_parser<'a, T>(
        input: &'a str,
        output: ParsedFeed,
        date: DateTime<FixedOffset>,
    ) -> Result<(), ParserError>
    where
        T: Parser<'a>,
    {
        let mut reader = Reader::from_str(input);
        let Event::Start(root) = reader.read_event()? else {
            return Err(ParserError::UnrecognizedRoot(None));
        };
        let parser = T::try_from_root(root)
            .map_err(|tag| ParserError::UnrecognizedRoot(Some(Box::from(tag.local_name()))))?;

        assert_eq!(parser.parse(&mut reader, date)?, output);

        Ok(())
    }

    #[test]
    fn test_decode_text_to_end() -> Result<(), ParserError> {
        assert!(matches!(
            decode_text_to_end(&mut Reader::from_str("&lt;/link<![CDATA[>]]>"), "p")?,
            Cow::Owned(s) if s == "</link>",
        ));

        assert!(matches!(
            decode_text_to_end(&mut Reader::from_str("foo"), "p")?,
            Cow::Borrowed("foo"),
        ));
        assert!(matches!(
            decode_text_to_end(&mut Reader::from_str(""), "p")?,
            Cow::Borrowed(""),
        ));

        let mut reader = Reader::from_str("<p>&lt;/link<![CDATA[>]]></p>");
        reader.read_event()?;
        assert!(matches!(
            decode_text_to_end(&mut reader, "p")?,
            Cow::Owned(s) if s == "</link>",
        ));

        Ok(())
    }
}
