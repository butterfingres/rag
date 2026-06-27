use {
    crate::{
        borrow::Cow,
        xml::{ParserError, Replaceable, read_to_end},
    },
    allocator_api2::alloc::Allocator,
    jiff::{
        Timestamp,
        fmt::{rfc2822, temporal},
    },
    quick_xml::{XmlVersion, name::QName, reader::NsReader},
    std::{marker::PhantomData, str::FromStr},
};

pub trait TagParser<'alloc, 'src, A>
where
    A: Allocator,
{
    type Output;

    fn parse_tag(
        &self,
        _: &mut NsReader<&'src [u8]>,
        _: QName<'_>,
        _: XmlVersion,
        _: &'alloc A,
    ) -> Result<Self::Output, ParserError>;

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
    {
        Flatten(self)
    }

    fn flat_map<F, T>(self, f: F) -> Flatten<Map<F, Self>>
    where
        F: Fn(Self::Output) -> Result<T, ParserError>,
        Self: Sized,
    {
        self.map::<F>(f).flatten()
    }

    fn map<F>(self, f: F) -> Map<F, Self>
    where
        Self: Sized,
    {
        Map { f, parser: self }
    }
    fn map_from_str<T>(self) -> MapFromStr<T, Self>
    where
        Self: Sized,
    {
        MapFromStr {
            parser: self,
            _marker: PhantomData,
        }
    }
    fn map_replaceable(
        self,
        replaceable: bool,
    ) -> Map<impl Fn(Self::Output) -> Replaceable<Self::Output>, Self>
    where
        Self: Sized,
    {
        self.map(move |data| Replaceable { data, replaceable })
    }
}

pub struct Content;
impl<'alloc, 'src, A> TagParser<'alloc, 'src, A> for Content
where
    A: Allocator + 'alloc,
{
    type Output = Cow<'src, [u8], &'alloc A>;

    fn parse_tag(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        _: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self::Output, ParserError> {
        read_to_end(reader, name, alloc)
    }
}

pub struct Flatten<P>(P);
impl<'alloc, 'src, P, T, A> TagParser<'alloc, 'src, A> for Flatten<P>
where
    P: TagParser<'alloc, 'src, A, Output = Result<T, ParserError>>,
    A: Allocator,
{
    type Output = T;

    fn parse_tag(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self::Output, ParserError> {
        self.0.parse_tag(reader, name, version, alloc).flatten()
    }
}

pub struct Map<F, P> {
    f: F,
    parser: P,
}
impl<'alloc, 'src, F, P, T, A> TagParser<'alloc, 'src, A> for Map<F, P>
where
    F: Fn(P::Output) -> T,
    P: TagParser<'alloc, 'src, A>,
    A: Allocator,
{
    type Output = T;

    fn parse_tag(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self::Output, ParserError> {
        self.parser
            .parse_tag(reader, name, version, alloc)
            .map(&self.f)
    }
}
pub struct MapFromStr<T, P> {
    parser: P,
    _marker: PhantomData<T>,
}
impl<'alloc, 'src, P, S, T, U, A> TagParser<'alloc, 'src, A> for MapFromStr<T, P>
where
    P: TagParser<'alloc, 'src, A, Output = S>,
    S: AsRef<str>,
    T: FromStr<Err = U>,
    U: Into<ParserError>,
    A: Allocator,
{
    type Output = T;

    fn parse_tag(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self::Output, ParserError> {
        let input = self.parser.parse_tag(reader, name, version, alloc)?;
        T::from_str(input.as_ref()).map_err(<U as Into<ParserError>>::into)
    }
}
pub fn rfc2822_timestamp<T>(ts: T) -> Result<Timestamp, ParserError>
where
    T: AsRef<[u8]>,
{
    rfc2822::DateTimeParser::new()
        .parse_timestamp(ts)
        .map_err(ParserError::ParseTimestamp)
}
pub fn rfc3339_timestamp<T>(ts: T) -> Result<Timestamp, ParserError>
where
    T: AsRef<[u8]>,
{
    temporal::DateTimeParser::new()
        .parse_timestamp(ts)
        .map_err(ParserError::ParseTimestamp)
}
