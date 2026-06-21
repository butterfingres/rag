use {
    allocator_api2::alloc::Allocator,
    bitvec::BitArr,
    jiff::{SpanFieldwise, Timestamp},
    quick_xml::{
        events::{BytesStart, Event},
        reader::NsReader,
    },
    std::{
        borrow::Cow,
        error::Error,
        fmt::{self, Display, Formatter},
        num::NonZeroU16,
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
// impl<'a> PartialText<'a> {
//     pub const fn strong(text: Cow<'a, str>) -> Self {
//         Self {
//             text,
//             authority: Authority::Strong,
//         }
//     }
//     pub const fn weak(text: Cow<'a, str>) -> Self {
//         Self {
//             text,
//             authority: Authority::Weak,
//         }
//     }

//     fn should_replace(old: &Option<Self>, authority: Authority) -> bool {
//         old.is_none() || old.as_ref().is_some_and(|old| authority > old.authority)
//     }
//     pub fn replace_with_text_or_skip(
//         text: &mut Option<Self>,
//         tag: &str,
//         reader: &mut NsReader<'a>,
//         authority: Authority,
//     ) -> Result<(), ParserError> {
//         if Self::should_replace(text, authority) {
//             *text = Some(Self {
//                 text: decode_text_to_end(reader, tag)?,
//                 authority,
//             });
//             Ok(())
//         } else {
//             reader
//                 .read_to_end(tag)
//                 .map(|_| ())
//                 .map_err(ParserError::Xml)
//         }
//     }
//     pub fn replace_text(old: &mut Option<Self>, new: Self) {
//         if Self::should_replace(old, new.authority) {
//             *old = Some(new);
//         }
//     }
// }
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
    Xml(quick_xml::Error),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Xml(e) => e.fmt(f),
        }
    }
}
impl Error for ParserError {}
impl From<quick_xml::Error> for ParserError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}

pub trait XmlParser<'a>: Sized {
    type State;

    fn try_from_root(_: BytesStart<'a>) -> Result<Self, BytesStart<'a>>;
    fn handle_event<A>(
        self,
        _: &mut NsReader<&'a str>,
        _: Event<'a>,
        _: &mut Self::State,
        _: &'a A,
    ) -> Result<Self, ParserError>
    where
        A: Allocator + ?Sized;
}
