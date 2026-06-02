use {
    chrono::{DateTime, FixedOffset},
    quick_xml::{
        XmlVersion,
        encoding::{Decoder, EncodingError},
        events::{BytesStart, Event},
        name::ResolveResult,
        reader::NsReader,
    },
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
    },
    tokio::io::AsyncBufRead,
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

pub struct PartialFeed<T> {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: Option<DateTime<FixedOffset>>,
    /// Extra metadata to add to the feed.
    meta: T,
}
pub struct Feed {
    pub title: Box<str>,
    // The link is optional in atom.
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: DateTime<FixedOffset>,
}

pub struct PartialEntry<T> {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub pub_date: Option<DateTime<FixedOffset>>,
    pub fetch_date: DateTime<FixedOffset>,
    pub enclosures: Vec<Box<str>>,
    meta: T,
}
pub type Entry = PartialEntry<()>;

pub struct ParsedFeed {
    pub feed: Feed,
    pub entries: Vec<Entry>,
}

pub enum Namespace {
    Dc,
    Sy,
}
impl Namespace {
    pub fn new(ns: ResolveResult, decoder: Decoder) -> Result<Option<Self>, EncodingError> {
        if let ResolveResult::Bound(quick_xml::name::Namespace(bytes)) = ns {
            match decoder.decode(bytes)?.as_ref() {
                "dc" => Ok(Some(Self::Dc)),
                "sy" => Ok(Some(Self::Sy)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub enum ParserError {
    Encoding(EncodingError),
    Xml(quick_xml::Error),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Encoding(e) => e.fmt(f),
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
    fn try_new<'a>(_: BytesStart<'a>) -> Result<Self, BytesStart<'a>>;
    fn handle_event<T>(
        self,
        _: (Option<Namespace>, Event<'_>),
        _: &mut NsReader<T>,
        _: &mut Vec<u8>,
        _: &XmlVersion,
    ) -> impl Future<Output = Result<Self, ParserError>>
    where
        T: AsyncBufRead + Unpin;

    fn parse<T>(
        mut self,
        reader: &mut NsReader<T>,
        ev_buf: &mut Vec<u8>,
        text_buf: &mut Vec<u8>,
        version: &XmlVersion,
    ) -> impl Future<Output = Result<ParsedFeed, ParserError>>
    where
        T: AsyncBufRead + Unpin,
        ParsedFeed: TryFrom<Self, Error = ParserError>,
    {
        async {
            let decoder = reader.decoder();
            loop {
                match reader.read_resolved_event_into_async(ev_buf).await? {
                    (_, Event::Eof) => break ParsedFeed::try_from(self),
                    (ns, ev) => {
                        let ns = Namespace::new(ns, decoder)?;
                        self = self
                            .handle_event((ns, ev), reader, text_buf, version)
                            .await?;
                    }
                }
            }
        }
    }
}
