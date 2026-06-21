use {
    crate::borrow::Cow,
    allocator_api2::{alloc::Allocator, collections::TryReserveError},
    bitvec::BitArr,
    jiff::{SpanFieldwise, Timestamp},
    quick_xml::{
        escape::resolve_xml_entity,
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
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<ReplaceableText<'a, A>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
#[derive(Debug, PartialEq)]
pub struct Feed<'a, A>
where
    A: Allocator + ?Sized,
{
    pub title: Cow<'a, [u8], &'a A>,
    // The link is optional in atom.
    pub link: Option<Cow<'a, [u8], &'a A>>,
    pub cache: Cache,
    pub last_update: Option<Timestamp>,
}
impl<'a, A> Feed<'a, A>
where
    A: Allocator + ?Sized,
{
    pub fn from_partial(
        PartialFeed {
            title,
            link,
            cache,
            last_update,
        }: PartialFeed<'a, A>,
    ) -> Option<Self> {
        Some(Self {
            title: title?,
            link: link.map(Cow::from),
            cache,
            last_update,
        })
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
pub struct ReplaceableText<'a, A>
where
    A: Allocator + ?Sized,
{
    text: Cow<'a, [u8], &'a A>,
    replaceable: bool,
}
// impl<'a> ReplaceableText<'a> {
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
impl<'a, A> From<ReplaceableText<'a, A>> for Cow<'a, [u8], &'a A>
where
    A: Allocator + ?Sized,
{
    fn from(ReplaceableText { text, .. }: ReplaceableText<'a, A>) -> Cow<'a, [u8], &'a A> {
        text
    }
}

#[derive(Default)]
pub struct PartialEntry<'a, A>
where
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<ReplaceableText<'a, A>>,
    pub description: Option<ReplaceableText<'a, A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, [u8], &'a A>>,
}

#[derive(Debug, PartialEq)]
pub struct Entry<'a, A>
where
    A: Allocator + ?Sized,
{
    pub title: Option<Cow<'a, [u8], &'a A>>,
    pub link: Option<Cow<'a, [u8], &'a A>>,
    pub description: Option<Cow<'a, [u8], &'a A>>,
    pub pub_date: Option<Timestamp>,
    pub enclosures: Vec<Cow<'a, [u8], &'a A>>,
}
impl<'a, A> From<PartialEntry<'a, A>> for Entry<'a, A>
where
    A: Allocator + ?Sized,
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
    A: Allocator + ?Sized,
{
    pub feed: Feed<'a, A>,
    pub entries: Vec<Entry<'a, A>>,
}

#[derive(Debug)]
pub enum ParserError {
    MissingRoot,
    TryReserve(TryReserveError),
    Xml(quick_xml::Error),
}
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::MissingRoot => f.write_str("failed to get root element"),
            Self::TryReserve(e) => e.fmt(f),
            Self::Xml(e) => e.fmt(f),
        }
    }
}
impl Error for ParserError {}
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

pub trait XmlParser<'a>: Sized {
    type State;

    fn try_from_root(_: BytesStart<'a>) -> Result<Self, BytesStart<'a>>;
    fn handle_event<A>(
        self,
        _: &mut NsReader<&'a [u8]>,
        _: Event<'a>,
        _: &mut Self::State,
        _: &'a A,
    ) -> Result<Self, ParserError>
    where
        A: Allocator + ?Sized;
}
macro_rules! xml_parser {
    (
        $ident:ident {
            $($var:ident),* $(,)?
        },
        [$($pat:pat => $expr:expr),* $(,)?]
    ) => {
        pub struct $ident;

        impl $crate::xml::XmlParser for $ident {
            fn try_from_root(_: BytesStart<'a>) -> Result<Self, BytesStart<'a>> {
                todo!()
            }
            fn handle_event<A>(
                self,
                _: &mut NsReader<&'a [u8]>,
                _: Event<'a>,
                _: &mut Self::State,
                _: &'a A,
            ) -> Result<Self, ParserError>
            where
                A: Allocator + ?Sized,
            {
                todo!()
            }
        }
    };
}

fn read_to_end<'a, A>(
    reader: &mut NsReader<&'a [u8]>,
    name: QName<'a>,
    alloc: &'a A,
) -> Result<Cow<'a, [u8], &'a A>, ParserError>
where
    A: Allocator + ?Sized,
{
    let mut output = Cow::Borrowed(&b""[..]);

    loop {
        match reader.read_event()? {
            Event::Text(text) => match output {
                Cow::Borrowed(b"") => {
                    output = Cow::try_from_global_in(text.into_inner(), alloc)?;
                }
                _ => {
                    output.try_to_mut_in(alloc)?.extend(text.iter());
                }
            },
            Event::CData(text) => match output {
                Cow::Borrowed(b"") => {
                    output = Cow::try_from_global_in(text.into_inner(), alloc)?;
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

    Ok(output)
}

pub trait HandleElement<'a, A>
where
    A: Allocator + ?Sized,
{
    type State;

    fn handle_element(
        _: &mut Self::State,
        _: &mut NsReader<&'a [u8]>,
        _: QName<'a>,
        _: &'a A,
    ) -> Result<(), ParserError>;
}

trait IsReplaceable {
    const IS_REPLACEABLE: bool;
}
struct Replaceable;
impl IsReplaceable for Replaceable {
    const IS_REPLACEABLE: bool = true;
}
struct Unreplaceable;
impl IsReplaceable for Unreplaceable {
    const IS_REPLACEABLE: bool = false;
}

#[expect(private_bounds)]
pub struct ReplaceableTextHandler<T>
where
    T: IsReplaceable,
{
    _marker: PhantomData<T>,
}
impl<'a, T, A> HandleElement<'a, A> for ReplaceableTextHandler<T>
where
    T: IsReplaceable,
    A: Allocator + ?Sized + 'a,
{
    type State = ReplaceableText<'a, A>;

    fn handle_element(
        text: &mut ReplaceableText<'a, A>,
        reader: &mut NsReader<&'a [u8]>,
        name: QName<'a>,
        alloc: &'a A,
    ) -> Result<(), ParserError> {
        if let ReplaceableText {
            replaceable: true, ..
        } = text
        {
            *text = ReplaceableText {
                text: read_to_end(reader, name, alloc)?,
                replaceable: T::IS_REPLACEABLE,
            };
            Ok(())
        } else {
            Ok(())
        }
    }
}

struct StringHandler;
impl<'a, A> HandleElement<'a, A> for StringHandler
where
    A: Allocator + ?Sized + 'a,
{
    type State = Cow<'a, [u8], &'a A>;

    fn handle_element(
        text: &mut Cow<'a, [u8], &'a A>,
        reader: &mut NsReader<&'a [u8]>,
        name: QName<'a>,
        alloc: &'a A,
    ) -> Result<(), ParserError> {
        *text = read_to_end(reader, name, alloc)?;
        Ok(())
    }
}

struct OptionalHandler<H> {
    _marker: PhantomData<H>,
}
impl<'a, H, A> HandleElement<'a, A> for OptionalHandler<H>
where
    H: HandleElement<'a, A>,
    H::State: Default,
    A: Allocator,
{
    type State = Option<H::State>;

    fn handle_element(
        option: &mut Self::State,
        reader: &mut NsReader<&'a [u8]>,
        name: QName<'a>,
        alloc: &'a A,
    ) -> Result<(), ParserError> {
        if option.is_none() {
            let mut val = Default::default();
            H::handle_element(&mut val, reader, name, alloc)?;
            *option = Some(val);
            Ok(())
        } else {
            reader.read_to_end(name)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::alloc, std::assert_matches, stumpalo::Arena};

    fn test_read_to_end<A, F>(input: &str, alloc: &A, f: F) -> Result<(), ParserError>
    where
        A: Allocator + ?Sized,
        F: FnOnce(Cow<'_, [u8], &A>),
    {
        let mut reader = NsReader::from_str(input);
        loop {
            match reader.read_event()? {
                Event::Start(tag) => {
                    f(read_to_end(&mut reader, tag.name(), alloc)?);
                    return Ok(());
                }
                Event::Eof => return Err(ParserError::MissingRoot),
                _ => {}
            }
        }
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
        let mut alloc = Arena::new();

        test_read_to_end(
            "<p>&lt;b&gt;hello world&lt;/b&gt;</p>",
            &alloc,
            |val| assert_matches!(val, Cow::Owned(val) if val == b"<b>hello world</b>"),
        )?;
        alloc.clear();

        test_read_to_end(
            "<p>&lt;b&gt;hello world<![CDATA[ goodbye world]]>&lt;/b&gt;</p>",
            &alloc,
            |val| assert_matches!(val, Cow::Owned(val) if val == b"<b>hello world goodbye world</b>"),
        )?;
        alloc.clear();

        Ok(())
    }
}
