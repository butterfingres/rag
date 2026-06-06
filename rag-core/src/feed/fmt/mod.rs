pub mod atom;
pub mod rss;
pub mod rss_1_0;

use {
    crate::{
        rfc822,
        utf8::{Event, Reader, Start},
    },
    bitvec::BitArr,
    jiff::{SpanFieldwise, Timestamp},
    quick_xml::{escape::resolve_xml_entity, events::attributes::AttrError},
    std::{
        borrow::Cow,
        error::Error,
        fmt::{self, Display, Formatter},
        num::{NonZeroU16, ParseIntError, TryFromIntError},
        str,
    },
};

pub type SkipWeekdays = BitArr![for 7, in u8];
pub type SkipHours = BitArr![for 24, in u32];

#[derive(Debug, PartialEq)]
pub struct Period {
    interval: SpanFieldwise,
    base: Option<Timestamp>,
    // A frequency of 0 means nothing.
    frequency: NonZeroU16,
}

#[derive(Debug, Default, PartialEq)]
pub struct Cache {
    skip_weekdays: SkipWeekdays,
    skip_hours: SkipHours,
    period: Option<Period>,
}

#[derive(Default)]
pub struct PartialFeed<'a> {
    pub title: Option<Cow<'a, str>>,
    pub link: Option<PartialText<'a>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
#[derive(Debug, PartialEq)]
pub struct Feed<'a> {
    pub title: Cow<'a, str>,
    // The link is optional in atom.
    pub link: Option<Cow<'a, str>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
impl<'a> Feed<'a> {
    pub fn from_partial(
        PartialFeed {
            title,
            link,
            cache,
            last_update,
        }: PartialFeed<'a>,
    ) -> Option<Self> {
        Some(Self {
            title: title?,
            link: link.map(Cow::<'a, str>::from),
            cache,
            last_update,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum Authority {
    #[default]
    Weak,
    Strong,
}

/// Text content that may come from multiple sources, with differing
/// reliablility.
///
/// You should use this over a standard [Cow] whenever there are
/// multiple sources for the same information such as with links and
/// descriptions where their quality can differ. Otherwise, you should
/// stick to a normal type and always override it.
#[derive(Debug, PartialEq)]
pub struct PartialText<'a> {
    text: Cow<'a, str>,
    authority: Authority,
}
impl<'a> PartialText<'a> {
    pub const fn strong(text: Cow<'a, str>) -> Self {
        Self {
            text,
            authority: Authority::Strong,
        }
    }
    pub const fn weak(text: Cow<'a, str>) -> Self {
        Self {
            text,
            authority: Authority::Weak,
        }
    }

    fn should_replace(old: &Option<Self>, authority: Authority) -> bool {
        old.is_none() || old.as_ref().is_some_and(|old| authority > old.authority)
    }
    pub fn replace_with_text_or_skip(
        text: &mut Option<Self>,
        tag: &str,
        reader: &mut Reader<'a>,
        authority: Authority,
    ) -> Result<(), ParserError> {
        if Self::should_replace(text, authority) {
            *text = Some(Self {
                text: decode_text_to_end(reader, tag)?,
                authority,
            });
            Ok(())
        } else {
            reader
                .read_to_end(tag)
                .map(|_| ())
                .map_err(ParserError::Xml)
        }
    }
    pub fn replace_text(old: &mut Option<Self>, new: Self) {
        if Self::should_replace(old, new.authority) {
            *old = Some(new);
        }
    }
}
impl<'a> From<PartialText<'a>> for Cow<'a, str> {
    fn from(PartialText { text, .. }: PartialText<'a>) -> Cow<'a, str> {
        text
    }
}

#[derive(Default)]
pub struct PartialEntry<'a> {
    pub title: Option<Cow<'a, str>>,
    pub link: Option<PartialText<'a>>,
    pub description: Option<PartialText<'a>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, str>>,
}

#[derive(Debug, PartialEq)]
pub struct Entry<'a> {
    pub title: Option<Cow<'a, str>>,
    pub link: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, str>>,
}
impl<'a> From<PartialEntry<'a>> for Entry<'a> {
    fn from(
        PartialEntry {
            title,
            link,
            description,
            pub_date,
            enclosures,
        }: PartialEntry<'a>,
    ) -> Self {
        Self {
            title,
            link: link.map(Cow::<'a, str>::from),
            description: description.map(Cow::<'a, str>::from),
            pub_date,
            enclosures,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ParsedFeed<'a> {
    pub feed: Feed<'a>,
    pub entries: Vec<Entry<'a>>,
}

#[derive(Debug)]
pub enum ParserError {
    Invalid,
    ParseInt(ParseIntError),
    // TODO: merge ParseTime and ParseWeekday
    ParseTime(jiff::Error),
    ParseWeekday(Box<str>),
    Rfc822(rfc822::Error),
    Xml(quick_xml::Error),
    TryFromInt(TryFromIntError),
    UnrecognizedRoot(Option<Box<str>>),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Invalid => f.write_str("the feed does not conform to specifications"),
            Self::ParseInt(e) => e.fmt(f),
            Self::ParseTime(e) => e.fmt(f),
            Self::ParseWeekday(day) => write!(f, "failed to parse weekday `{day}`"),
            Self::Rfc822(e) => e.fmt(f),
            Self::Xml(e) => e.fmt(f),
            Self::TryFromInt(e) => e.fmt(f),
            Self::UnrecognizedRoot(Some(tag)) => write!(f, "unrecognized root element `{tag}`"),
            Self::UnrecognizedRoot(None) => f.write_str("failed to get root element"),
        }
    }
}
impl Error for ParserError {}
impl From<AttrError> for ParserError {
    fn from(e: AttrError) -> Self {
        Self::Xml(quick_xml::Error::InvalidAttr(e))
    }
}
impl From<ParseIntError> for ParserError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
impl From<rfc822::Error> for ParserError {
    fn from(e: rfc822::Error) -> Self {
        Self::Rfc822(e)
    }
}
impl From<jiff::Error> for ParserError {
    fn from(e: jiff::Error) -> Self {
        Self::ParseTime(e)
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
    fn output(self) -> Option<ParsedFeed<'a>>;
    fn handle_event(self, _: Event<'a>, _: &mut Reader<'a>) -> Result<Self, ParserError>;

    fn parse(mut self, reader: &mut Reader<'a>) -> Result<ParsedFeed<'a>, ParserError> {
        loop {
            match reader.read_event()? {
                Event::Eof => break self.output().ok_or(ParserError::Invalid),
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

    pub fn test_parser<'a, T>(input: &'a str, output: ParsedFeed) -> Result<(), ParserError>
    where
        T: Parser<'a>,
    {
        let mut reader = Reader::from_str(input);

        let root = loop {
            match reader.read_event()? {
                Event::Start(root) => break root,
                Event::Eof => return Err(ParserError::UnrecognizedRoot(None)),
                _ => {}
            }
        };
        let parser = T::try_from_root(root)
            .map_err(|tag| ParserError::UnrecognizedRoot(Some(Box::from(tag.local_name()))))?;

        assert_eq!(parser.parse(&mut reader)?, output);

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

    fn test_replacement(
        mut input: Option<PartialText>,
        output: Option<PartialText>,
        authority: Authority,
    ) -> Result<(), ParserError> {
        let mut reader = Reader::from_str("<p>hello world</p>");
        reader.read_event()?;

        PartialText::replace_with_text_or_skip(&mut input, "p", &mut reader, authority)?;
        assert_eq!(input, output);

        Ok(())
    }

    #[test]
    fn test_replacement_empty() -> Result<(), ParserError> {
        test_replacement(
            None,
            Some(PartialText {
                text: Cow::Borrowed("hello world"),
                authority: Authority::Weak,
            }),
            Authority::Weak,
        )?;
        test_replacement(
            None,
            Some(PartialText {
                text: Cow::Borrowed("hello world"),
                authority: Authority::Strong,
            }),
            Authority::Strong,
        )?;

        Ok(())
    }

    #[test]
    fn test_replacement_overpower() -> Result<(), ParserError> {
        test_replacement(
            Some(PartialText {
                text: Cow::Borrowed("weak text"),
                authority: Authority::Weak,
            }),
            Some(PartialText {
                text: Cow::Borrowed("hello world"),
                authority: Authority::Strong,
            }),
            Authority::Strong,
        )
    }

    #[test]
    fn test_replacement_lazy() -> Result<(), ParserError> {
        test_replacement(
            Some(PartialText {
                text: Cow::Borrowed("weak text"),
                authority: Authority::Weak,
            }),
            Some(PartialText {
                text: Cow::Borrowed("weak text"),
                authority: Authority::Weak,
            }),
            Authority::Weak,
        )?;

        test_replacement(
            Some(PartialText {
                text: Cow::Borrowed("strong text"),
                authority: Authority::Strong,
            }),
            Some(PartialText {
                text: Cow::Borrowed("strong text"),
                authority: Authority::Strong,
            }),
            Authority::Strong,
        )?;
        test_replacement(
            Some(PartialText {
                text: Cow::Borrowed("strong text"),
                authority: Authority::Strong,
            }),
            Some(PartialText {
                text: Cow::Borrowed("strong text"),
                authority: Authority::Strong,
            }),
            Authority::Weak,
        )?;

        Ok(())
    }
}
