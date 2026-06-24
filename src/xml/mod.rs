pub mod atom;
pub mod rss_2_0;

use {
    crate::{borrow::Cow, num::ParseIntError},
    allocator_api2::{
        alloc::{AllocError, Allocator},
        collections::TryReserveError,
        vec::Vec,
    },
    bitvec::BitArr,
    jiff::{
        Timestamp,
        fmt::{rfc2822, temporal},
    },
    quick_xml::{
        errors::SyntaxError,
        escape::resolve_xml_entity,
        events::attributes::AttrError,
        events::{BytesStart, Event},
        name::QName,
        reader::{NsReader, Reader, Span},
    },
    std::{
        error::Error,
        fmt::{self, Debug, Display, Formatter},
        marker::PhantomData,
        ops::Range,
        str,
    },
};

pub type SkipDays = BitArr![for 7, in u8];
pub type SkipHours = BitArr![for 24, in u32];

#[derive(Debug, PartialEq)]
pub struct Feed<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'src, [u8], &'alloc A>>,
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub skip_days: SkipDays,
    pub skip_hours: SkipHours,
    pub last_update: Option<Timestamp>,
}

/// Text content that may come from multiple sources, with differing
/// reliablility.
///
/// You should use this over a standard [Cow] whenever there are
/// multiple sources for the same information such as with links and
/// descriptions where their quality can differ. Otherwise, you should
/// stick to a normal type and always override it.
#[derive(Debug, PartialEq)]
pub struct Replaceable<T> {
    data: T,
    replaceable: bool,
}
impl<T> Default for Replaceable<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            data: T::default(),
            replaceable: true,
        }
    }
}
impl<T> Replaceable<T> {
    fn into_inner(Replaceable { data, .. }: Replaceable<T>) -> T {
        data
    }
}

#[derive(Debug, PartialEq)]
pub struct Enclosure<'src> {
    tag: BytesStart<'src>,
    enclosure: Range<usize>,
}

pub struct Entry<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'src, [u8], &'alloc A>>,
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub description: Option<Cow<'src, [u8], &'alloc A>>,
    pub id: Option<Cow<'src, [u8], &'alloc A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Enclosure<'src>, &'alloc A>,
}
impl<'alloc, 'src, A> Debug for Entry<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Entry")
            .field("title", &self.title)
            .field("link", &self.link)
            .field("description", &self.description)
            .field("pub_date", &self.pub_date)
            .field("enclosures", &self.enclosures)
            .finish()
    }
}
impl<A, B> PartialEq<Entry<'_, '_, B>> for Entry<'_, '_, A>
where
    A: Allocator + ?Sized,
    B: Allocator + ?Sized,
{
    fn eq(
        &self,
        Entry {
            title,
            link,
            description,
            id,
            pub_date,
            enclosures,
        }: &Entry<'_, '_, B>,
    ) -> bool {
        self.title.as_deref() == title.as_deref()
            && self.link.as_deref() == link.as_deref()
            && self.description.as_deref() == description.as_deref()
            && self.id.as_deref() == id.as_deref()
            && self.pub_date == *pub_date
            && self.enclosures == *enclosures
    }
}

#[derive(Debug, PartialEq)]
pub struct ParsedFeed<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    pub feed: Feed<'alloc, 'src, A>,
    pub entries: Vec<Entry<'alloc, 'src, A>>,
}

#[derive(Debug)]
pub enum ParserError {
    Alloc(AllocError),
    MissingRoot,
    NotUtf8,
    ParseInt(ParseIntError),
    ParseTimestamp(jiff::Error),
    TryReserve(TryReserveError),
    UnknownWeekday,
    Xml(quick_xml::Error),
}
impl ParserError {
    const UNCLOSED_TAG: Self = Self::Xml(quick_xml::Error::Syntax(SyntaxError::UnclosedTag));
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Alloc(e) => Display::fmt(e, f),
            Self::MissingRoot => f.write_str("failed to get root element"),
            Self::NotUtf8 => f.write_str("input is not utf8"),
            Self::ParseInt(e) => Display::fmt(e, f),
            Self::ParseTimestamp(e) => Display::fmt(e, f),
            Self::TryReserve(e) => Display::fmt(e, f),
            Self::UnknownWeekday => f.write_str("unknown weekday"),
            Self::Xml(e) => Display::fmt(e, f),
        }
    }
}
impl Error for ParserError {}
impl From<AllocError> for ParserError {
    fn from(e: AllocError) -> Self {
        Self::Alloc(e)
    }
}
impl From<bump_scope::alloc::AllocError> for ParserError {
    fn from(_: bump_scope::alloc::AllocError) -> Self {
        Self::Alloc(AllocError)
    }
}
impl From<jiff::Error> for ParserError {
    fn from(e: jiff::Error) -> Self {
        Self::ParseTimestamp(e)
    }
}
impl From<ParseIntError> for ParserError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
impl From<TryReserveError> for ParserError {
    fn from(e: TryReserveError) -> Self {
        Self::TryReserve(e)
    }
}
impl From<quick_xml::Error> for ParserError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}

#[derive(Debug)]
pub enum TryFromRootError<'src> {
    Attr(AttrError),
    UnknownRoot(BytesStart<'src>),
}
impl From<AttrError> for TryFromRootError<'_> {
    fn from(e: AttrError) -> Self {
        Self::Attr(e)
    }
}

pub trait ParserReader<'src> {
    fn from_str(_: &'src str) -> Self;

    fn read_event(&mut self) -> Result<Event<'src>, quick_xml::Error>;
    fn read_to_end(&mut self, _: QName<'_>) -> Result<Span, quick_xml::Error>;
}
impl<'src> ParserReader<'src> for Reader<&'src [u8]> {
    fn from_str(input: &'src str) -> Self {
        Self::from_str(input)
    }
    fn read_event(&mut self) -> Result<Event<'src>, quick_xml::Error> {
        self.read_event()
    }
    fn read_to_end(&mut self, end: QName<'_>) -> Result<Span, quick_xml::Error> {
        self.read_to_end(end)
    }
}
impl<'src> ParserReader<'src> for NsReader<&'src [u8]> {
    fn from_str(input: &'src str) -> Self {
        Self::from_str(input)
    }
    fn read_event(&mut self) -> Result<Event<'src>, quick_xml::Error> {
        self.read_event()
    }
    fn read_to_end(&mut self, end: QName<'_>) -> Result<Span, quick_xml::Error> {
        self.read_to_end(end)
    }
}

pub trait Parser<'alloc, 'src, A>: Sized
where
    Self: Sized,
    A: Allocator + ?Sized,
{
    type Reader: ParserReader<'src>;
    type State;

    fn try_from_root(_: BytesStart<'src>, _: &Self::Reader)
    -> Result<Self, TryFromRootError<'src>>;
    fn handle_event<F>(
        self,
        _: &mut Self::Reader,
        _: Event<'src>,
        _: &mut Self::State,
        _: F,
        _: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>;
    fn handle_events<F>(
        mut self,
        reader: &mut Self::Reader,
        mut cb: F,
        alloc: &'alloc A,
    ) -> Result<Self::State, ParserError>
    where
        Self::State: Default,
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        let mut state = Default::default();
        loop {
            match reader.read_event()? {
                Event::Eof => break Ok(state),
                event => self = self.handle_event(reader, event, &mut state, &mut cb, alloc)?,
            }
        }
    }
}

fn read_to_end<'alloc, 'src, R, A>(
    reader: &mut R,
    name: QName<'_>,
    alloc: &'alloc A,
) -> Result<Cow<'src, [u8], &'alloc A>, ParserError>
where
    R: ParserReader<'src>,
    A: Allocator + ?Sized,
{
    let mut output = Cow::Borrowed(&b""[..]);
    read_to_end_in(reader, name, &mut output, alloc)?;
    Ok(output)
}

fn read_to_end_in<'alloc, 'src, R, A>(
    reader: &mut R,
    name: QName<'_>,
    output: &mut Cow<'src, [u8], &'alloc A>,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    R: ParserReader<'src> + ?Sized,
    A: Allocator + ?Sized,
{
    *output = Cow::Borrowed(b"");

    loop {
        match reader.read_event()? {
            Event::Text(text) => {
                match output {
                    Cow::Borrowed(b"") => {
                        *output = Cow::try_from_in(text.into_inner(), alloc)?;
                    }
                    _ => {
                        output
                            .try_to_mut_in(alloc)?
                            .extend_from_slice(text.as_ref());
                    }
                };
            }
            Event::CData(text) => match output {
                Cow::Borrowed(b"") => {
                    *output = Cow::try_from_in(text.into_inner(), alloc)?;
                }
                _ => {
                    output
                        .try_to_mut_in(alloc)?
                        .extend_from_slice(text.as_ref());
                }
            },
            Event::GeneralRef(ch) => {
                if let Some(ch) = ch.resolve_char_ref()? {
                    let mut buf = [0; 4];
                    output
                        .try_to_mut_in(alloc)?
                        .extend(ch.encode_utf8(&mut buf).bytes());
                } else if let Some(ch) = str::from_utf8(ch.as_ref())
                    .ok()
                    .and_then(resolve_xml_entity)
                {
                    output.try_to_mut_in(alloc)?.extend(ch.bytes());
                }
            }
            Event::Start(start) => {
                reader.read_to_end(start.name())?;
            }
            Event::End(end) if end.name() == name => return Ok(()),
            Event::Eof => return Err(ParserError::UNCLOSED_TAG),
            _ => {}
        }
    }
}

pub trait HandleElementInto<'alloc, 'src, R, A, S = Self>
where
    R: ParserReader<'src>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        _: &mut S,
        _: &mut R,
        _: QName<'_>,
        _: &'alloc A,
    ) -> Result<(), ParserError>;
}

pub struct CallbackHandler<F, T, U> {
    _marker: PhantomData<(F, T, U)>,
}
impl<'alloc, 'src, F, R, T, U, A> HandleElementInto<'alloc, 'src, R, A, F>
    for CallbackHandler<F, T, U>
where
    F: FnMut(U) -> Result<(), ParserError>,
    R: ParserReader<'src>,
    T: HandleElementInto<'alloc, 'src, R, A, U>,
    U: Default,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        closure: &mut F,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut val = U::default();
        T::handle_element_into(&mut val, reader, name, alloc)?;
        closure(val)?;

        Ok(())
    }
}

pub struct ReplaceableHandler<const REPLACEABLE: bool, T, U = T> {
    _marker: PhantomData<(T, U)>,
}
impl<'alloc, 'src, const REPLACEABLE: bool, R, T, U, A>
    HandleElementInto<'alloc, 'src, R, A, Replaceable<U>> for ReplaceableHandler<REPLACEABLE, T, U>
where
    R: ParserReader<'src>,
    T: HandleElementInto<'alloc, 'src, R, A, U>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        replaceable: &mut Replaceable<U>,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        if let Replaceable {
            replaceable: replaceable @ true,
            data,
        } = replaceable
        {
            T::handle_element_into(data, reader, name, alloc)?;
            *replaceable = REPLACEABLE;
            Ok(())
        } else {
            reader.read_to_end(name)?;
            Ok(())
        }
    }
}

impl<'alloc, 'src, R, A> HandleElementInto<'alloc, 'src, R, A> for Cow<'src, [u8], &'alloc A>
where
    R: ParserReader<'src>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        into: &mut Cow<'src, [u8], &'alloc A>,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        read_to_end_in(reader, name, into, alloc)
    }
}

pub struct OptionHandler<T, U = T> {
    _marker: PhantomData<(T, U)>,
}
impl<'alloc, 'src, R, T, U, A> HandleElementInto<'alloc, 'src, R, A, Option<U>>
    for OptionHandler<T, U>
where
    R: ParserReader<'src>,
    T: HandleElementInto<'alloc, 'src, R, A, U>,
    U: Default,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        option: &mut Option<U>,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        if let Some(val) = option {
            T::handle_element_into(val, reader, name, alloc)?;
            Ok(())
        } else {
            let mut val = U::default();
            T::handle_element_into(&mut val, reader, name, alloc)?;
            *option = Some(val);
            Ok(())
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Rfc2822Timestamp(Timestamp);
impl From<Timestamp> for Rfc2822Timestamp {
    fn from(ts: Timestamp) -> Self {
        Self(ts)
    }
}
impl From<Rfc2822Timestamp> for Timestamp {
    fn from(Rfc2822Timestamp(ts): Rfc2822Timestamp) -> Self {
        ts
    }
}
impl<'alloc, 'src, R, A> HandleElementInto<'alloc, 'src, R, A> for Rfc2822Timestamp
where
    R: ParserReader<'src>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        timestamp: &mut Rfc2822Timestamp,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let new_timestamp = read_to_end(reader, name, alloc)?;
        let new_timestamp = rfc2822::DateTimeParser::new().parse_timestamp(&new_timestamp)?;
        *timestamp = Rfc2822Timestamp(new_timestamp);
        Ok(())
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Rfc3339Timestamp(Timestamp);
impl From<Timestamp> for Rfc3339Timestamp {
    fn from(ts: Timestamp) -> Self {
        Self(ts)
    }
}
impl From<Rfc3339Timestamp> for Timestamp {
    fn from(Rfc3339Timestamp(ts): Rfc3339Timestamp) -> Self {
        ts
    }
}
impl<'alloc, 'src, R, A> HandleElementInto<'alloc, 'src, R, A> for Rfc3339Timestamp
where
    R: ParserReader<'src>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        timestamp: &mut Rfc3339Timestamp,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let new_timestamp = read_to_end(reader, name, alloc)?;
        let new_timestamp = temporal::DateTimeParser::new().parse_timestamp(&new_timestamp)?;
        *timestamp = Rfc3339Timestamp(new_timestamp);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::alloc,
        allocator_api2::alloc::Global,
        bump_scope::Bump,
        std::{assert_matches, fmt::Debug},
    };

    fn get_root<'src, R>(reader: &mut R) -> Result<BytesStart<'src>, ParserError>
    where
        R: ParserReader<'src>,
    {
        loop {
            match reader.read_event()? {
                Event::Start(tag) => break Ok(tag),
                Event::Eof => break Err(ParserError::MissingRoot),
                _ => {}
            }
        }
    }

    #[expect(
        dead_code,
        reason = "the data is used by the [Debug] implementation which is printed on error cases"
    )]
    #[derive(Debug)]
    pub enum TestParserError<'a> {
        Parser(ParserError),
        TryFromRoot(TryFromRootError<'a>),
    }
    impl<T> From<T> for TestParserError<'_>
    where
        T: Into<ParserError>,
    {
        fn from(e: T) -> Self {
            Self::Parser(e.into())
        }
    }
    impl<'a> From<TryFromRootError<'a>> for TestParserError<'a> {
        fn from(e: TryFromRootError<'a>) -> Self {
            Self::TryFromRoot(e)
        }
    }
    pub fn test_parser<'alloc, 'src, const N: usize, T, A>(
        input: &'src str,
        output_state: T::State,
        output_entries: [Entry<'alloc, 'src, A>; N],
        alloc: &'alloc A,
    ) -> Result<(), TestParserError<'src>>
    where
        T: Parser<'alloc, 'src, A>,
        T::State: Debug + Default + PartialEq,
        A: Allocator + ?Sized,
    {
        let mut reader = T::Reader::from_str(input);
        let root = get_root(&mut reader)?;

        let mut items = 0;

        let parser = T::try_from_root(root, &reader)?;
        let state = parser.handle_events(
            &mut reader,
            |entry| {
                assert_eq!(entry, output_entries[items]);
                items += 1;
                Ok(())
            },
            alloc,
        )?;
        assert_eq!(state, output_state);
        assert_eq!(N, items);

        Ok(())
    }

    fn test_read_to_end<A, F>(input: &str, alloc: &A, f: F) -> Result<(), ParserError>
    where
        A: Allocator + ?Sized,
        F: FnOnce(Cow<'_, [u8], &A>),
    {
        let mut reader = NsReader::from_str(input);
        let root = get_root(&mut reader)?;
        f(read_to_end(&mut reader, root.name(), alloc)?);
        Ok(())
    }

    #[test]
    fn read_to_end_borrowed() -> Result<(), ParserError> {
        test_read_to_end("<p>hello world</p>", &alloc::Dummy, |val| {
            assert_matches!(val, Cow::Borrowed(b"hello world"))
        })?;
        test_read_to_end(
            "<p><![CDATA[<b>hello</b> world]]></p>",
            &alloc::Dummy,
            |val| assert_matches!(val, Cow::Borrowed(b"<b>hello</b> world")),
        )?;

        Ok(())
    }

    #[test]
    fn read_to_end_owned() -> Result<(), ParserError> {
        const LONGEST: &[u8] = b"<b>hello world goodbye world</b>";

        let mut alloc = Bump::<Global>::try_with_size(LONGEST.len())?;

        test_read_to_end(
            "<p>&lt;b&gt;hello world&lt;/b&gt;</p>",
            &alloc,
            |val| assert_matches!(val, Cow::Owned(val) if val == b"<b>hello world</b>"),
        )?;
        alloc.reset();

        test_read_to_end(
            "<p>&lt;b&gt;hello world<![CDATA[ goodbye world]]>&lt;/b&gt;</p>",
            &alloc,
            |val| assert_matches!(val, Cow::Owned(val) if val == LONGEST),
        )?;

        Ok(())
    }
}
