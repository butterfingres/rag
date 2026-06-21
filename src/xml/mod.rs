use {
    crate::borrow::Cow,
    allocator_api2::alloc::Allocator,
    bitvec::BitArr,
    jiff::{SpanFieldwise, Timestamp},
    quick_xml::{
        events::{BytesStart, Event},
        reader::NsReader,
    },
    std::{
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
pub struct PartialFeed<'a, A>
where
    A: Allocator,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<PartialText<'a, &'a A>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
#[derive(Debug, PartialEq)]
pub struct Feed<'a, A>
where
    A: Allocator,
{
    pub title: Cow<'a, [u8], &'a A>,
    // The link is optional in atom.
    pub link: Option<Cow<'a, [u8], &'a A>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
// impl<'a, A> Feed<'a, A>
// where
//     A: Allocator,
// {
//     pub fn from_partial(
//         PartialFeed {
//             title,
//             link,
//             cache,
//             last_update,
//         }: PartialFeed<'a, A>,
//     ) -> Option<Self> {
//         Some(Self {
//             title: title?,
//             link: link.map(Cow::from),
//             cache,
//             last_update,
//         })
//     }
// }

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
pub struct PartialText<'a, A>
where
    A: Allocator,
{
    text: Cow<'a, [u8], &'a A>,
    authority: Authority,
}
// impl<'a> PartialText<'a> {
//     pub const fn strong(text: Cow<'a, [u8], A>) -> Self {
//         Self {
//             text,
//             authority: Authority::Strong,
//         }
//     }
//     pub const fn weak(text: Cow<'a, [u8], A>) -> Self {
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
impl<'a, A> From<PartialText<'a, A>> for Cow<'a, [u8], &'a A>
where
    A: Allocator,
{
    fn from(PartialText { text, .. }: PartialText<'a, A>) -> Cow<'a, [u8], &'a A> {
        text
    }
}

#[derive(Default)]
pub struct PartialEntry<'a, A>
where
    A: Allocator,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<PartialText<'a, A>>,
    pub description: Option<PartialText<'a, A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, [u8], &'a A>>,
}

#[derive(Debug, PartialEq)]
pub struct Entry<'a, A>
where
    A: Allocator,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<Cow<'a, [u8], &'a A>>,
    pub description: Option<Cow<'a, [u8], &'a A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, [u8], &'a A>>,
}
impl<'a, A> From<PartialEntry<'a, A>> for Entry<'a, A>
where
    A: Allocator,
{
    fn from(
        PartialEntry {
            title,
            link,
            description,
            pub_date,
            enclosures,
        }: PartialEntry<'a, A>,
    ) -> Self {
        Self {
            title,
            link: link.map(Cow::from),
            description: description.map(Cow::from),
            pub_date,
            enclosures,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ParsedFeed<'a, A>
where
    A: Allocator,
{
    pub feed: Feed<'a, A>,
    pub entries: Vec<Entry<'a, A>>,
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
        A: Allocator;
}
