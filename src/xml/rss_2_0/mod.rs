#[cfg(test)]
mod tests;

use {
    crate::{
        borrow::Cow,
        num,
        xml::{
            self, Entry, HandleElementInto, OptionHandler, ParserError, Replaceable,
            ReplaceableHandler, Rfc2822Timestamp, SkipDays, SkipHours, TryFromRootError,
            read_to_end,
        },
    },
    allocator_api2::alloc::Allocator,
    bitvec::{
        array::BitArray,
        order::{BitOrder, Lsb0},
        view::BitViewSized,
    },
    quick_xml::{
        events::{BytesStart, Event},
        name::QName,
        reader::NsReader,
    },
    std::{
        fmt::{self, Debug, Formatter},
        marker::PhantomData,
    },
};

trait RssSkip {
    const TAG: &str;

    type Order: BitOrder;
    type View: BitViewSized;
    type Index: Into<usize>;

    fn parse_index(_: &[u8]) -> Result<Self::Index, ParserError>;
}

struct RssSkipHour;
impl RssSkip for RssSkipHour {
    const TAG: &str = "hour";

    type Order = Lsb0;
    type View = [u32; 1];
    type Index = u8;

    fn parse_index(index: &[u8]) -> Result<Self::Index, ParserError> {
        num::parse(index).map_err(ParserError::ParseInt)
    }
}

struct RssSkipDay;
impl RssSkip for RssSkipDay {
    const TAG: &str = "day";

    type Order = Lsb0;
    type View = [u8; 1];
    type Index = u8;

    fn parse_index(index: &[u8]) -> Result<Self::Index, ParserError> {
        match index {
            b"Monday" => Ok(0),
            b"Tuesday" => Ok(1),
            b"Wednesday" => Ok(2),
            b"Thursday" => Ok(3),
            b"Friday" => Ok(4),
            b"Saturday" => Ok(5),
            b"Sunday" => Ok(6),
            _ => Err(ParserError::UnknownWeekday),
        }
    }
}

struct RssSkipHandler<T> {
    _marker: PhantomData<T>,
}
impl<'alloc, 'src, T, A> HandleElementInto<'alloc, 'src, A, BitArray<T::View, T::Order>>
    for RssSkipHandler<T>
where
    A: Allocator + ?Sized,
    T: RssSkip,
{
    fn handle_element_into(
        bitvec: &mut BitArray<T::View, T::Order>,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        loop {
            match reader.read_event()? {
                Event::Start(tag) if tag.name().0 == T::TAG.as_bytes() => {
                    let index = read_to_end(reader, tag.name(), alloc)?;
                    let index: usize = T::parse_index(index.as_ref())?.into();
                    bitvec.set(index, true);
                }
                Event::Start(tag) => {
                    reader.read_to_end(tag.name())?;
                }

                Event::End(tag) if tag.name() == name => return Ok(()),
                Event::Eof => {
                    return Err(ParserError::UNCLOSED_TAG);
                }

                _ => {}
            }
        }
    }
}

pub struct Channel<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Cow<'src, [u8], &'alloc A>>,
    modify_date: Option<Replaceable<Rfc2822Timestamp>>,
    skip_hours: SkipHours,
    skip_days: SkipDays,
}
impl<A> Debug for Channel<'_, '_, A>
where
    A: Allocator + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("channel")
            .field("title", &self.title)
            .field("link", &self.link)
            .field("modify_date", &self.modify_date)
            .finish()
    }
}
impl<'alloc, 'src, A> Default for Channel<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    fn default() -> Self {
        Self {
            title: None,
            link: None,
            modify_date: None,
            skip_hours: SkipHours::default(),
            skip_days: SkipDays::default(),
        }
    }
}
impl<'alloc, 'src, A> PartialEq for Channel<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    fn eq(&self, r: &Self) -> bool {
        self.title.as_ref() == r.title.as_ref()
            && self.link.as_ref() == r.link.as_ref()
            && self.modify_date == r.modify_date
    }
}

pub struct Item<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Cow<'src, [u8], &'alloc A>>,
    description: Option<Cow<'src, [u8], &'alloc A>>,
}
impl<A> Default for Item<'_, '_, A>
where
    A: Allocator + ?Sized,
{
    fn default() -> Self {
        Self {
            title: None,
            link: None,
            description: None,
        }
    }
}
impl<'alloc, 'src, A> From<Item<'alloc, 'src, A>> for Entry<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    fn from(
        Item {
            title,
            link,
            description,
        }: Item<'alloc, 'src, A>,
    ) -> Entry<'alloc, 'src, A> {
        Entry {
            title,
            link,
            description,
            ..Default::default()
        }
    }
}
impl<'alloc, 'src, F, T, A> HandleElementInto<'alloc, 'src, A, F> for Item<'alloc, 'src, A>
where
    F: FnMut(Entry<'alloc, 'src, A>) -> T,
    T: Into<Result<(), ParserError>>,
    A: Allocator + ?Sized,
{
    fn handle_element_into(
        cb: &mut F,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut item = Item::default();
        loop {
            match reader.read_event()? {
                Event::Start(tag) if tag.name().0 == b"title" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.title,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"link" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.link,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"description" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.description,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }

                Event::Start(tag) => {
                    reader.read_to_end(tag.name())?;
                }

                Event::End(tag) if tag.name() == name => {
                    cb(item.into()).into()?;
                    return Ok(());
                }
                Event::Eof => return Err(ParserError::UNCLOSED_TAG),

                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
}
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for Step
where
    A: Allocator + 'static,
{
    type State = Channel<'alloc, 'src, A>;
    fn try_from_root(tag: BytesStart<'src>) -> Result<Self, TryFromRootError<'src>> {
        if tag.name().0 == b"rss" && {
            let mut found = false;
            for attr in tag.attributes() {
                let attr = attr?;
                if attr.key.0 == b"version" && *attr.value == *b"2.0" {
                    found = true;
                    break;
                }
            }
            found
        } {
            Ok(Self::OutsideChannel)
        } else {
            Err(TryFromRootError::UnknownRoot(tag))
        }
    }
    fn handle_event<F>(
        self,
        reader: &mut NsReader<&'src [u8]>,
        event: Event<'src>,
        state: &mut Channel<'alloc, 'src, A>,
        mut cb: F,
        alloc: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        match (self, event) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::InsideChannel)
            }
            (Step::InsideChannel, Event::End(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::OutsideChannel)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"title" => {
                OptionHandler::<_>::handle_element_into(&mut state.title, reader, tag.name(), alloc)
                    .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"link" => {
                OptionHandler::<_>::handle_element_into(&mut state.link, reader, tag.name(), alloc)
                    .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"pubDate" => {
                OptionHandler::<ReplaceableHandler<true, _>, _>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"lastBuildDate" => {
                OptionHandler::<ReplaceableHandler<false, _>, _>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"skipHours" => {
                RssSkipHandler::<RssSkipHour>::handle_element_into(
                    &mut state.skip_hours,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"skipDays" => {
                RssSkipHandler::<RssSkipDay>::handle_element_into(
                    &mut state.skip_days,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }

            (Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"ttl" => {
                todo!()
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"item" => {
                Item::handle_element_into(&mut cb, reader, tag.name(), alloc).map(|_| step)
            }
            (step, Event::Start(tag)) => reader
                .read_to_end(tag.name())
                .map_err(ParserError::Xml)
                .map(|_| step),

            (step, _) => Ok(step),
        }
    }
}
