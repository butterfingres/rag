use {
    crate::{
        borrow::Cow,
        num,
        xml::{
            self, Enclosure, Entry, Feed, HandleElementInto, OptionHandler, ParserError,
            ParserReader, Replaceable, ReplaceableHandler, Rfc2822Timestamp, SkipDays, SkipHours,
            TryFromRootError, read_to_end,
        },
    },
    allocator_api2::{alloc::Allocator, vec::Vec},
    bitvec::{
        array::BitArray,
        order::{BitOrder, Lsb0},
        view::BitViewSized,
    },
    jiff::Timestamp,
    quick_xml::{
        events::{BytesStart, Event, attributes::Attribute},
        name::QName,
        reader::Reader,
    },
    std::{
        fmt::{self, Debug, Formatter},
        marker::PhantomData,
        ops::Deref,
        ptr,
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
impl<'alloc, 'src, T, R, A> HandleElementInto<'alloc, 'src, R, A, BitArray<T::View, T::Order>>
    for RssSkipHandler<T>
where
    T: RssSkip,
    R: ParserReader<'src>,
    A: Allocator,
{
    fn handle_element_into(
        bitvec: &mut BitArray<T::View, T::Order>,
        reader: &mut R,
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
    A: Allocator,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Cow<'src, [u8], &'alloc A>>,
    modify_date: Option<Replaceable<Rfc2822Timestamp>>,
    skip_hours: SkipHours,
    skip_days: SkipDays,
}
impl<A> Debug for Channel<'_, '_, A>
where
    A: Allocator,
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
    A: Allocator,
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
impl<'alloc, 'src, A> From<Channel<'alloc, 'src, A>> for Feed<'alloc, 'src, A>
where
    A: Allocator,
{
    fn from(
        Channel {
            title,
            link,
            modify_date,
            skip_hours,
            skip_days,
        }: Channel<'alloc, 'src, A>,
    ) -> Feed<'alloc, 'src, A> {
        Feed {
            title,
            link,
            skip_days,
            skip_hours,
            last_update: modify_date
                .map(Replaceable::into_inner)
                .map(Timestamp::from),
        }
    }
}
impl<'alloc, 'src, A> PartialEq for Channel<'alloc, 'src, A>
where
    A: Allocator,
{
    fn eq(&self, r: &Self) -> bool {
        self.title.as_ref() == r.title.as_ref()
            && self.link.as_ref() == r.link.as_ref()
            && self.modify_date == r.modify_date
    }
}

pub struct Item<'alloc, 'src, A>
where
    A: Allocator,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Cow<'src, [u8], &'alloc A>>,
    description: Option<Cow<'src, [u8], &'alloc A>>,
    id: Option<Cow<'src, [u8], &'alloc A>>,
    pub_date: Option<Rfc2822Timestamp>,
    enclosures: Vec<Enclosure<'src>, &'alloc A>,
}
impl<'alloc, 'src, A> Item<'alloc, 'src, A>
where
    A: Allocator,
{
    fn new_in(alloc: &'alloc A) -> Self {
        Self {
            title: None,
            link: None,
            description: None,
            id: None,
            pub_date: None,
            enclosures: Vec::new_in(alloc),
        }
    }
}
impl<'alloc, 'src, A> From<Item<'alloc, 'src, A>> for Entry<'alloc, 'src, A>
where
    A: Allocator,
{
    fn from(
        Item {
            title,
            link,
            description,
            id,
            pub_date,
            enclosures,
        }: Item<'alloc, 'src, A>,
    ) -> Entry<'alloc, 'src, A> {
        Entry {
            title,
            link,
            description,
            id,
            pub_date: pub_date.map(Timestamp::from),
            enclosures,
        }
    }
}
impl<'alloc, 'src, A> Item<'alloc, 'src, A>
where
    A: Allocator,
{
    fn handle_enclosure(&mut self, enclosure: BytesStart<'src>) -> Result<(), ParserError> {
        // HACK: we use pointer arithmetic and rely on the attribute
        // parser because we cannot borrow `enclosure` for 'src as
        // required by the signature of [BytesStart::attributes]. This
        // should be fine since our inputs must always be utf8 so
        // everything just be borrowing from the same slice.

        let attrs_start = ptr::from_ref(enclosure.deref()).addr();

        for attr in enclosure.attributes() {
            let Attribute {
                key: QName(key),
                value,
            } = attr.map_err(quick_xml::Error::InvalidAttr)?;
            // We don't need to escape the url because that would make
            // the url invalid.
            if key == b"url" {
                let std::borrow::Cow::Borrowed(value) = value else {
                    return Err(ParserError::NotUtf8);
                };

                let url_start = ptr::from_ref(value).addr() - attrs_start;
                let url_end = url_start + value.len();

                self.enclosures.push(Enclosure {
                    tag: enclosure,
                    enclosure: url_start..url_end,
                });
                break;
            }
        }

        Ok(())
    }
}
impl<'alloc, 'src, F, T, R, A> HandleElementInto<'alloc, 'src, R, A, F> for Item<'alloc, 'src, A>
where
    F: FnMut(Entry<'alloc, 'src, A>) -> T,
    R: ParserReader<'src>,
    T: Into<Result<(), ParserError>>,
    A: Allocator,
{
    fn handle_element_into(
        cb: &mut F,
        reader: &mut R,
        name: QName<'_>,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut item = Item::new_in(alloc);
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
                Event::Start(tag) if tag.name().0 == b"guid" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.id,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"pubDate" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.pub_date,
                        reader,
                        tag.name(),
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"enclosure" => {
                    reader.read_to_end(tag.name())?;
                    item.handle_enclosure(tag)?;
                }
                Event::Empty(tag) if tag.name().0 == b"enclosure" => {
                    item.handle_enclosure(tag)?;
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
    A: Allocator + 'alloc,
{
    type Reader = Reader<&'src [u8]>;
    type State = Channel<'alloc, 'src, A>;

    fn try_from_root(
        tag: BytesStart<'src>,
        _: &Self::Reader,
    ) -> Result<Self, TryFromRootError<'src>> {
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
        reader: &mut Self::Reader,
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

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc, tz,
            xml::tests::{TestParserError, test_parser},
        },
        allocator_api2::{alloc::Global, vec},
        jiff::civil::datetime,
    };

    #[test]
    fn test_rss_parser_all() -> Result<(), TestParserError<'static>> {
        test_parser::<_, Step, _>(
            include_str!("./all.xml"),
            Channel {
                title: Some(Cow::Borrowed(b"example feed")),
                link: Some(Cow::Borrowed(b"https://example.com/rss")),
                modify_date: Some(Replaceable {
                    // Fri, 21 Jul 2023 09:04 EDT
                    data: datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    replaceable: false,
                }),
                skip_hours: SkipHours::new([0b0111]),
                skip_days: SkipDays::new([0b0111]),
            },
            [Entry {
                title: Some(Cow::Borrowed(b"entry 1")),
                link: Some(Cow::Borrowed(b"https://example.com/entry_1")),
                description: Some(Cow::Borrowed(b"the first entry")),
                id: Some(Cow::Borrowed(b"1")),
                // Fri, 20 Jun 2003 09:00:00 GMT
                pub_date: datetime(2003, 06, 20, 09, 00, 00, 00)
                    .to_zoned(tz::GMT)?
                    .timestamp()
                    .into(),
                enclosures: vec![in &Global; Enclosure {
                    tag: BytesStart::from_content(r#"enclosure url="https://example.com/entry_1.mp3""#, "enclosure".len()),
                    enclosure: r#"enclosure url=""#.len()..r#"enclosure url="https://example.com/entry_1.mp3"#.len(),
                }],
            }],
            &Global,
        )
    }

    #[test]
    fn test_rss_parser_alt() -> Result<(), TestParserError<'static>> {
        test_parser::<_, Step, _>(
            include_str!("./alt.xml"),
            Channel {
                title: Some(Cow::Borrowed(b"example feed")),
                link: Some(Cow::Borrowed(b"https://example.com/rss")),
                modify_date: Some(Replaceable {
                    // Fri, 21 Jul 2023 09:04 EDT
                    data: datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    replaceable: false,
                }),
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::new([0b0111_1111]),
            },
            [],
            &alloc::Dummy,
        )
    }
}
