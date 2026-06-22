pub mod rss_2_0;

use {
    crate::{borrow::Cow, num::ParseIntError},
    allocator_api2::{
        alloc::{AllocError, Allocator},
        collections::TryReserveError,
    },
    bitvec::BitArr,
    jiff::{SpanFieldwise, Timestamp, fmt::rfc2822},
    quick_xml::{
        escape::resolve_xml_entity,
        events::attributes::AttrError,
        events::{BytesStart, Event},
        name::QName,
        reader::NsReader,
    },
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        num::NonZeroU16,
        str,
    },
};

#[derive(Debug, Default, PartialEq)]
pub struct SkipWeekdays(BitArr![for 7, in u8]);
#[derive(Debug, Default, PartialEq)]
pub struct SkipHours(BitArr![for 24, in u32]);

#[derive(Debug, PartialEq)]
pub struct Period {
    interval: SpanFieldwise,
    base: Option<Timestamp>,
    // A frequency of 0 means nothing.
    frequency: NonZeroU16,
}

#[derive(Debug, PartialEq)]
pub struct Feed<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    pub title: Cow<'src, [u8], &'alloc A>,
    // The link is optional in atom.
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub skip_weekdays: SkipWeekdays,
    pub skip_hours: SkipHours,
    pub period: Option<Period>,
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

#[derive(Debug, PartialEq)]
pub struct Entry<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'src, [u8], &'alloc A>>,
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub description: Option<Cow<'src, [u8], &'alloc A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'src, [u8], &'alloc A>>,
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
    ParseInt(ParseIntError),
    ParseTimestamp(jiff::Error),
    TryReserve(TryReserveError),
    Xml(quick_xml::Error),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Alloc(e) => e.fmt(f),
            Self::MissingRoot => f.write_str("failed to get root element"),
            Self::ParseInt(e) => e.fmt(f),
            Self::ParseTimestamp(e) => e.fmt(f),
            Self::TryReserve(e) => e.fmt(f),
            Self::Xml(e) => e.fmt(f),
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

pub trait Parser<'alloc, 'src, A>: Sized
where
    Self: Sized,
    A: Allocator + ?Sized,
{
    type State;

    fn try_from_root(_: BytesStart<'src>) -> Result<Self, TryFromRootError<'src>>;
    fn handle_event(
        self,
        _: &mut NsReader<&'src [u8]>,
        _: Event<'src>,
        _: &mut Self::State,
        _: &'alloc A,
    ) -> Result<Self, ParserError>;
    fn handle_events(
        mut self,
        reader: &mut NsReader<&'src [u8]>,
        alloc: &'alloc A,
    ) -> Result<Self::State, ParserError>
    where
        Self::State: Default,
    {
        let mut state = Default::default();
        loop {
            match reader.read_event()? {
                Event::Eof => break Ok(state),
                event => self = self.handle_event(reader, event, &mut state, alloc)?,
            }
        }
    }
}

fn get_root<'src>(reader: &mut NsReader<&'src [u8]>) -> Result<BytesStart<'src>, ParserError> {
    loop {
        match reader.read_event()? {
            Event::Start(tag) => break Ok(tag),
            Event::Eof => break Err(ParserError::MissingRoot),
            _ => {}
        }
    }
}

fn read_to_end<'alloc, 'src, A>(
    reader: &mut NsReader<&'src [u8]>,
    name: QName<'_>,
    alloc: &'alloc A,
) -> Result<Cow<'src, [u8], &'alloc A>, ParserError>
where
    A: Allocator + ?Sized,
{
    let mut output = Cow::Borrowed(&b""[..]);
    read_to_end_in(reader, name, &mut output, alloc)?;
    Ok(output)
}

fn read_to_end_in<'alloc, 'src, A>(
    reader: &mut NsReader<&'src [u8]>,
    name: QName<'_>,
    output: &mut Cow<'src, [u8], &'alloc A>,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    A: Allocator + ?Sized,
{
    match output {
        Cow::Borrowed(_) => *output = Cow::Borrowed(b""),
        Cow::Owned(buf) => buf.clear(),
    }

    loop {
        match reader.read_event()? {
            Event::Text(text) => match output {
                Cow::Borrowed(b"") => {
                    *output = Cow::try_from_global_in(text.into_inner(), alloc)?;
                }
                _ => {
                    output.try_to_mut_in(alloc)?.extend(text.iter());
                }
            },
            Event::CData(text) => match output {
                Cow::Borrowed(b"") => {
                    *output = Cow::try_from_global_in(text.into_inner(), alloc)?;
                }
                _ => {
                    output.try_to_mut_in(alloc)?.extend(text.iter());
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
                output.try_to_mut_in(alloc)?;
            }
            Event::End(end) if end.name() == name => break,
            _ => {
                output.try_to_mut_in(alloc)?;
            }
        }
    }

    Ok(())
}

pub trait HandleElementInto<'alloc, 'src, A, S = Self>
where
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        _: &mut S,
        _: &mut NsReader<&'src [u8]>,
        _: QName<'_>,
        _: &'alloc A,
    ) -> Result<(), ParserError>;
}

pub struct ReplaceableHandler<const REPLACEABLE: bool, T> {
    _marker: PhantomData<T>,
}
impl<'alloc, 'src, const REPLACEABLE: bool, T, A> HandleElementInto<'alloc, 'src, A, Replaceable<T>>
    for ReplaceableHandler<REPLACEABLE, T>
where
    T: HandleElementInto<'alloc, 'src, A>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        replaceable: &mut Replaceable<T>,
        reader: &mut NsReader<&'src [u8]>,
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

impl<'alloc, 'src, A> HandleElementInto<'alloc, 'src, A> for Cow<'src, [u8], &'alloc A>
where
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        into: &mut Cow<'src, [u8], &'alloc A>,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        read_to_end_in(reader, name, into, alloc)
    }
}

impl<'alloc, 'src, T, U, A> HandleElementInto<'alloc, 'src, A, Option<U>> for Option<T>
where
    T: HandleElementInto<'alloc, 'src, A, U>,
    U: Default,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        option: &mut Option<U>,
        reader: &mut NsReader<&'src [u8]>,
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
impl<'alloc, 'src, A> HandleElementInto<'alloc, 'src, A> for Rfc2822Timestamp
where
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        timestamp: &mut Rfc2822Timestamp,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let new_timestamp = read_to_end(reader, name, alloc)?;
        let new_timestamp = rfc2822::DateTimeParser::new().parse_timestamp(&new_timestamp)?;
        *timestamp = Rfc2822Timestamp(new_timestamp);
        Ok(())
    }
}
impl From<Timestamp> for Rfc2822Timestamp {
    fn from(ts: Timestamp) -> Self {
        Self(ts)
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
    pub fn test_parser<'alloc, 'src, T, A>(
        input: &'src str,
        output: T::State,
        alloc: &'alloc A,
    ) -> Result<(), TestParserError<'src>>
    where
        T: Parser<'alloc, 'src, A>,
        T::State: Debug + Default + PartialEq,
        A: Allocator + ?Sized,
    {
        let mut reader = NsReader::from_str(input);
        let root = get_root(&mut reader)?;

        let parser = T::try_from_root(root)?;
        let state = parser.handle_events(&mut reader, alloc)?;
        assert_eq!(state, output);

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
