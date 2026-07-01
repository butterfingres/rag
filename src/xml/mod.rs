pub mod fmt;
pub mod ns;
pub mod parser;

use {
    crate::{borrow::Cow, fmt::debug_iter_bytes, num::ParseIntError, sym, xml::parser::TagParser},
    allocator_api2::{
        alloc::{AllocError, Allocator},
        boxed::Box,
        collections::TryReserveError,
        vec::Vec,
    },
    arrayvec::ArrayVec,
    bitvec::BitArr,
    emacs::{FromLisp, IntoLisp},
    jiff::Timestamp,
    quick_xml::{
        XmlVersion,
        errors::SyntaxError,
        escape::resolve_xml_entity,
        events::{
            BytesStart, Event,
            attributes::{AttrError, Attribute},
        },
        name::QName,
        reader::NsReader,
    },
    std::{
        error::Error,
        fmt::{Debug, Display, Formatter},
        ptr,
        str::{self, Utf8Error},
    },
};

pub type SkipDays = BitArr![for 7, in u8];
pub type SkipHours = BitArr![for 24, in u32];

pub struct PartialFeed<'alloc, 'src, A>
where
    A: Allocator,
{
    title: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    link: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    last_update: Replaceable<Option<Timestamp>>,
    skip_hours: SkipHours,
    skip_days: SkipDays,
    ttl: Option<u64>,
}
impl<'alloc, 'src, A> Default for PartialFeed<'alloc, 'src, A>
where
    A: Allocator,
{
    fn default() -> Self {
        Self {
            title: Replaceable::default(),
            link: Replaceable::default(),
            last_update: Replaceable::default(),
            skip_hours: SkipHours::default(),
            skip_days: SkipDays::default(),
            ttl: None,
        }
    }
}
impl<'alloc, 'src, A> From<PartialFeed<'alloc, 'src, A>> for Feed<'alloc, 'src, A>
where
    A: Allocator,
{
    fn from(
        PartialFeed {
            title,
            link,
            last_update,
            skip_hours,
            skip_days,
            ttl,
        }: PartialFeed<'alloc, 'src, A>,
    ) -> Feed<'alloc, 'src, A> {
        Feed {
            title: title.data,
            link: link.data,
            skip_days,
            skip_hours,
            ttl,
            last_update: last_update.data,
        }
    }
}

pub struct Feed<'alloc, 'src, A>
where
    A: Allocator,
{
    pub title: Option<Cow<'src, [u8], &'alloc A>>,
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub skip_days: SkipDays,
    pub skip_hours: SkipHours,
    pub ttl: Option<u64>,
    pub last_update: Option<Timestamp>,
}
impl<A> Debug for Feed<'_, '_, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let Self {
            title,
            link,
            last_update,
            skip_hours,
            skip_days,
            ttl,
        } = self;
        f.debug_struct("Feed")
            .field("title", &title)
            .field("link", &link)
            .field("last_update", &last_update)
            .field("skip_hours", &skip_hours)
            .field("skip_days", &skip_days)
            .field("ttl", &ttl)
            .finish()
    }
}
impl<'e, A> IntoLisp<'e> for Feed<'_, '_, A>
where
    A: Allocator,
{
    fn into_lisp(self, env: &'e emacs::Env) -> Result<emacs::Value<'e>, emacs::Error> {
        let Self {
            title,
            link,
            skip_days,
            skip_hours,
            ttl,
            last_update,
        } = self;

        let mut args = ArrayVec::<emacs::Value<'e>, { 6 * 2 }>::new();
        if let Some(val) = title {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::TITLE.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = link {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::LINK.bind(env));
            args.push(val.into_lisp(env)?);
        }

        args.push(sym::key::SKIP_DAYS.bind(env));
        args.push(skip_days.data[0].into_lisp(env)?);

        args.push(sym::key::SKIP_HOURS.bind(env));
        args.push(skip_hours.data[0].into_lisp(env)?);

        if let Some(val) = ttl {
            args.push(sym::key::TTL.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = last_update {
            args.push(sym::key::LAST_UPDATE.bind(env));
            args.push(val.as_second().into_lisp(env)?);
        }

        sym::val::MAKE_RAG_FEED.call(env, args.as_ref())
    }
}
impl<A1, A2> PartialEq<Feed<'_, '_, A2>> for Feed<'_, '_, A1>
where
    A1: Allocator,
    A2: Allocator,
{
    fn eq(
        &self,
        Feed {
            title,
            link,
            last_update,
            skip_hours,
            skip_days,
            ttl,
        }: &Feed<'_, '_, A2>,
    ) -> bool {
        self.title.as_deref() == title.as_deref()
            && self.link.as_deref() == link.as_deref()
            && self.last_update == *last_update
            && self.skip_hours == *skip_hours
            && self.skip_days == *skip_days
            && self.ttl == *ttl
    }
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
impl<T> Replaceable<T> {
    fn new_replaceable(data: T) -> Self {
        Self {
            data,
            replaceable: true,
        }
    }
    fn new_irreplaceable(data: T) -> Self {
        Self {
            data,
            replaceable: false,
        }
    }

    fn try_replace_or_skip<'alloc, 'src, P, U, A>(
        &mut self,
        parser: P,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError>
    where
        P: TagParser<'alloc, 'src, A, Output = U>,
        U: Into<Replaceable<T>>,
        A: Allocator,
    {
        if self.replaceable {
            *self = parser.parse_tag(reader, name, version, alloc)?.into();
        } else {
            reader.read_to_end(name)?;
        }

        Ok(())
    }
    fn try_replace_with<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce() -> Result<Self, E>,
    {
        if self.replaceable {
            *self = f()?;
        }

        Ok(())
    }

    fn map<F, U>(self, f: F) -> Replaceable<U>
    where
        F: FnOnce(T) -> U,
    {
        Replaceable {
            data: f(self.data),
            replaceable: self.replaceable,
        }
    }
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

pub struct PartialEntry<'alloc, 'src, A>
where
    A: Allocator,
{
    title: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    link: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    content: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    id: Replaceable<Option<Cow<'src, [u8], &'alloc A>>>,
    updated: Replaceable<Option<Timestamp>>,
    enclosures: Vec<Box<[u8], &'alloc A>, &'alloc A>,
}
impl<'alloc, 'src, A> PartialEntry<'alloc, 'src, A>
where
    A: Allocator,
{
    fn new_in(alloc: &'alloc A) -> Self {
        Self {
            title: Replaceable::default(),
            link: Replaceable::default(),
            content: Replaceable::default(),
            id: Replaceable::default(),
            updated: Replaceable::default(),
            enclosures: Vec::new_in(alloc),
        }
    }
}

pub struct Entry<'alloc, 'src, A>
where
    A: Allocator,
{
    pub title: Option<Cow<'src, [u8], &'alloc A>>,
    pub link: Option<Cow<'src, [u8], &'alloc A>>,
    pub description: Option<Cow<'src, [u8], &'alloc A>>,
    pub id: Option<Cow<'src, [u8], &'alloc A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Box<[u8], &'alloc A>, &'alloc A>,
}
impl<'alloc, 'src, A> Debug for Entry<'alloc, 'src, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let Self {
            title,
            link,
            description,
            id,
            pub_date,
            enclosures,
        } = self;
        f.debug_struct("Entry")
            .field("title", &title)
            .field("link", &link)
            .field("description", &description)
            .field("id", &id)
            .field("pub_date", &pub_date)
            .field(
                "enclosures",
                &std::fmt::from_fn(|f| debug_iter_bytes(&enclosures, f)),
            )
            .finish()
    }
}
impl<'e, A> IntoLisp<'e> for Entry<'_, '_, A>
where
    A: Allocator,
{
    fn into_lisp(self, env: &'e emacs::Env) -> Result<emacs::Value<'e>, emacs::Error> {
        let Self {
            title,
            link,
            description,
            id,
            pub_date,
            enclosures,
        } = self;

        let mut args = ArrayVec::<emacs::Value, { 6 * 2 }>::new();

        if let Some(val) = title {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::TITLE.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = link {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::LINK.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = description {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::DESCRIPTION.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = id {
            let val = str::from_utf8(&val).unwrap();
            args.push(sym::key::ID.bind(env));
            args.push(val.into_lisp(env)?);
        }

        if let Some(val) = pub_date {
            args.push(sym::key::PUB_DATE.bind(env));
            args.push(val.as_second().into_lisp(env)?);
        }

        if !enclosures.is_empty() {
            args.push(sym::key::ENCLOSURES.bind(env));
            let buf =
                emacs::Vector::from_lisp(sym::fun::MAKE_VECTOR.call(env, (enclosures.len(), 0))?)?;
            for (i, enclosure) in enclosures.into_iter().enumerate() {
                buf.set(i, str::from_utf8(&enclosure).unwrap().into_lisp(env)?)?;
            }
            args.push(buf.into_lisp(env)?);
        }

        sym::val::MAKE_RAG_ENTRY.call(env, args.as_ref())
    }
}
impl<'alloc, 'src, A> From<PartialEntry<'alloc, 'src, A>> for Entry<'alloc, 'src, A>
where
    A: Allocator,
{
    fn from(
        PartialEntry {
            title,
            link,
            content,
            id,
            updated,
            enclosures,
        }: PartialEntry<'alloc, 'src, A>,
    ) -> Entry<'alloc, 'src, A> {
        Entry {
            title: title.data,
            link: link.data,
            description: content.data,
            id: id.data,
            pub_date: updated.data,
            enclosures,
        }
    }
}
impl<A, B> PartialEq<Entry<'_, '_, B>> for Entry<'_, '_, A>
where
    A: Allocator,
    B: Allocator,
    for<'a> allocator_api2::boxed::Box<[u8], &'a A>:
        PartialEq<allocator_api2::boxed::Box<[u8], &'a B>>,
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
    A: Allocator,
{
    pub feed: Feed<'alloc, 'src, A>,
    pub entries: Vec<Entry<'alloc, 'src, A>>,
}

#[derive(Debug)]
pub enum ParserError {
    Alloc(AllocError),
    DateOutOfRange,
    Emacs(emacs::Error),
    Jiff(jiff::Error),
    MissingRoot,
    ParseInt(ParseIntError),
    Utf8(Utf8Error),
    TryReserve(TryReserveError),
    UnknownWeekday,
    Xml(quick_xml::Error),
}
impl ParserError {
    const UNCLOSED_TAG: Self = Self::Xml(quick_xml::Error::Syntax(SyntaxError::UnclosedTag));
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Alloc(e) => Display::fmt(e, f),
            Self::DateOutOfRange => f.write_str("date is out of range"),
            Self::Emacs(e) => Display::fmt(e, f),
            Self::Jiff(e) => Display::fmt(e, f),
            Self::MissingRoot => f.write_str("failed to get root element"),
            Self::ParseInt(e) => Display::fmt(e, f),
            Self::TryReserve(e) => Display::fmt(e, f),
            Self::Utf8(e) => Display::fmt(e, f),
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
impl From<emacs::Error> for ParserError {
    fn from(e: emacs::Error) -> Self {
        Self::Emacs(e)
    }
}
impl From<jiff::Error> for ParserError {
    fn from(e: jiff::Error) -> Self {
        Self::Jiff(e)
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
impl From<Utf8Error> for ParserError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e)
    }
}
impl From<AttrError> for ParserError {
    fn from(e: AttrError) -> Self {
        Self::Xml(quick_xml::Error::InvalidAttr(e))
    }
}
impl From<quick_xml::Error> for ParserError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}

#[derive(Debug)]
pub enum TryFromRootError<'src> {
    Xml(quick_xml::Error),
    UnknownRoot(BytesStart<'src>),
}
impl From<AttrError> for TryFromRootError<'_> {
    fn from(e: AttrError) -> Self {
        Self::Xml(quick_xml::Error::InvalidAttr(e))
    }
}
impl From<quick_xml::Error> for TryFromRootError<'_> {
    fn from(e: quick_xml::Error) -> Self {
        Self::Xml(e)
    }
}

pub trait Parser<'alloc, 'src, A>: Sized
where
    Self: Sized,
    A: Allocator,
{
    fn try_from_root(
        _: BytesStart<'src>,
        _: &NsReader<&'src [u8]>,
        _: XmlVersion,
    ) -> Result<Self, TryFromRootError<'src>>;
    fn handle_event<F>(
        self,
        _: &mut NsReader<&'src [u8]>,
        _: Event<'src>,
        _: &mut PartialFeed<'alloc, 'src, A>,
        _: F,
        _: XmlVersion,
        _: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>;
    fn handle_events<F>(
        mut self,
        reader: &mut NsReader<&'src [u8]>,
        mut cb: F,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Feed<'alloc, 'src, A>, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        let mut state = PartialFeed::default();
        loop {
            match reader.read_event()? {
                Event::Eof => break Ok(state.into()),
                event => {
                    self = self.handle_event(reader, event, &mut state, &mut cb, version, alloc)?
                }
            }
        }
    }
}

fn get_attribute_when<'alloc, 'src, F, G, A>(
    tag: &'src BytesStart<'src>,
    mut early_exit: F,
    mut pred: G,
    version: XmlVersion,
    alloc: &'alloc A,
) -> Result<Option<Box<[u8], &'alloc A>>, ParserError>
where
    F: FnMut(&Attribute<'src>) -> Result<bool, ParserError>,
    G: FnMut(&Attribute<'src>) -> bool,
    A: Allocator,
{
    let mut value = None;
    let mut exit = false;
    for attr in tag.attributes() {
        let attr = attr?;
        if !exit {
            exit = early_exit(&attr)?;
        }
        if value.is_none() && pred(&attr) {
            value = Some(attr.normalized_value(version)?);
        }

        if value.is_some() && exit {
            break;
        }
    }

    let Some(value) = value else {
        return Ok(None);
    };

    let mut buf = Box::<[u8], _>::try_new_uninit_slice_in(value.len(), alloc)?;

    let value_ptr = value.as_ref().as_ptr();
    let buf_ptr = buf.as_mut_ptr().cast::<u8>();
    let value_len = value.len();
    // SAFETY: `buf` is a slice with size `len` and is guaranteed to be unique.
    unsafe {
        ptr::copy_nonoverlapping(value_ptr, buf_ptr, value_len);
    }

    // SAFETY: copying the buffer should initialize the bytes
    let buf = unsafe { buf.assume_init() };
    Ok(Some(buf))
}

fn read_to_end<'alloc, 'src, A>(
    reader: &mut NsReader<&'src [u8]>,
    name: QName<'_>,
    alloc: &'alloc A,
) -> Result<Cow<'src, [u8], &'alloc A>, ParserError>
where
    A: Allocator,
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
    A: Allocator,
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

pub fn get_header<'src>(
    reader: &mut NsReader<&'src [u8]>,
) -> Result<(XmlVersion, BytesStart<'src>), ParserError> {
    let mut version = None;

    loop {
        match reader.read_event()? {
            Event::Decl(decl) => version = Some(decl.xml_version()?),
            Event::Start(tag) => break Ok((version.unwrap_or_default(), tag)),
            Event::Eof => break Err(ParserError::MissingRoot),
            _ => {}
        }
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
        output_state: Feed<'alloc, 'src, A>,
        output_entries: [Entry<'alloc, 'src, A>; N],
        alloc: &'alloc A,
    ) -> Result<(), TestParserError<'src>>
    where
        T: Parser<'alloc, 'src, A>,
        A: Allocator,
    {
        let mut reader = NsReader::from_str(input);
        let (version, root) = get_header(&mut reader)?;

        let mut items = 0;

        let parser = T::try_from_root(root, &reader, version)?;
        let state = parser.handle_events(
            &mut reader,
            |entry| {
                assert_eq!(Some(&entry), output_entries.get(items));
                items += 1;
                Ok(())
            },
            version,
            alloc,
        )?;
        assert_eq!(state, output_state);
        assert_eq!(N, items);

        Ok(())
    }

    fn test_read_to_end<A, F>(input: &str, alloc: &A, f: F) -> Result<(), ParserError>
    where
        A: Allocator,
        F: FnOnce(Cow<'_, [u8], &A>),
    {
        let mut reader = NsReader::from_str(input);
        let (_, root) = get_header(&mut reader)?;
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
