use {
    crate::{
        borrow::Cow,
        fmt::debug_bytes,
        num,
        xml::{
            self, Entry, Feed, HandleElementInto, OptionHandler, ParserError, ParserReader,
            Replaceable, ReplaceableHandler, Rfc2822Timestamp, SkipDays, SkipHours,
            TryFromRootError, UintHandler, get_attribute_when, read_to_end,
        },
    },
    allocator_api2::{alloc::Allocator, boxed::Box, vec::Vec},
    bitvec::{
        array::BitArray,
        order::{BitOrder, Lsb0},
        view::BitViewSized,
    },
    jiff::Timestamp,
    quick_xml::{
        XmlVersion,
        events::{BytesStart, Event},
        name::QName,
        reader::Reader,
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
        _: XmlVersion,
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
    ttl: Option<u64>,
}
impl<A> Debug for Channel<'_, '_, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        let Self {
            title,
            link,
            modify_date,
            skip_hours,
            skip_days,
            ttl,
        } = self;
        f.debug_struct("Channel")
            .field(
                "title",
                &title
                    .as_ref()
                    .map(|title| fmt::from_fn(move |f| debug_bytes(&title, f))),
            )
            .field(
                "link",
                &link
                    .as_ref()
                    .map(|link| fmt::from_fn(move |f| debug_bytes(&link, f))),
            )
            .field("modify_date", &modify_date)
            .field("skip_hours", &skip_hours)
            .field("skip_days", &skip_days)
            .field("ttl", &ttl)
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
            ttl: None,
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
            ttl: _,
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
    fn eq(
        &self,
        Self {
            title,
            link,
            modify_date,
            skip_hours,
            skip_days,
            ttl,
        }: &Self,
    ) -> bool {
        self.title.as_ref() == title.as_ref()
            && self.link.as_ref() == link.as_ref()
            && self.modify_date == *modify_date
            && self.skip_hours == *skip_hours
            && self.skip_days == *skip_days
            && self.ttl == *ttl
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
    enclosures: Vec<Box<[u8], &'alloc A>, &'alloc A>,
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
    fn handle_enclosure(
        &mut self,
        enclosure: BytesStart<'src>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        if let Some(enclosure) = get_attribute_when(
            &enclosure,
            |_| Ok(true),
            |attr| attr.key.0 == b"url",
            version,
            alloc,
        )? {
            self.enclosures.push(enclosure);
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
        version: XmlVersion,
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
                        version,
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"link" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.link,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"description" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.description,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"guid" => {
                    let mut is_permalink = None;
                    for attr in tag.attributes() {
                        let attr = attr?;
                        if attr.key.0 == b"isPermalink" {
                            is_permalink = Some(attr.value.as_ref() == b"true");
                            break;
                        }
                    }

                    let mut link = Cow::Borrowed(&b""[..]);
                    Cow::<'src, [u8], &'alloc A>::handle_element_into(
                        &mut link,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                    if is_permalink.unwrap_or(true) && item.link.is_none() {
                        item.link = Some(link.clone_in(alloc)?);
                    }
                    item.id = Some(link);
                }
                Event::Start(tag) if tag.name().0 == b"pubDate" => {
                    OptionHandler::<_>::handle_element_into(
                        &mut item.pub_date,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                Event::Start(tag) if tag.name().0 == b"enclosure" => {
                    reader.read_to_end(tag.name())?;
                    item.handle_enclosure(tag, version, alloc)?;
                }
                Event::Empty(tag) if tag.name().0 == b"enclosure" => {
                    item.handle_enclosure(tag, version, alloc)?;
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
        version: XmlVersion,
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
                OptionHandler::<_>::handle_element_into(
                    &mut state.title,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"link" => {
                OptionHandler::<_>::handle_element_into(
                    &mut state.link,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"pubDate" => {
                OptionHandler::<ReplaceableHandler<true, _>, _>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"lastBuildDate" => {
                OptionHandler::<ReplaceableHandler<false, _>, _>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"skipHours" => {
                RssSkipHandler::<RssSkipHour>::handle_element_into(
                    &mut state.skip_hours,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"skipDays" => {
                RssSkipHandler::<RssSkipDay>::handle_element_into(
                    &mut state.skip_days,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"ttl" => {
                OptionHandler::<UintHandler<_>, _>::handle_element_into(
                    &mut state.ttl,
                    reader,
                    tag.name(),
                    version,
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"item" => {
                Item::handle_element_into(&mut cb, reader, tag.name(), version, alloc).map(|_| step)
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
        bump_scope::Bump,
        jiff::civil::datetime,
    };

    #[test]
    fn test_rss_parser_all() -> Result<(), TestParserError<'static>> {
        let alloc = Bump::<Global>::try_new()?;
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
                skip_hours: SkipHours::new([0b1110]),
                skip_days: SkipDays::new([0b0111_1111]),
                ttl: Some(30),
            },
            [
                Entry {
                    title: Some(Cow::Borrowed(b"entry 1")),
                    link: Some(Cow::Borrowed(b"https://example.com/entry_1")),
                    description: Some(Cow::Borrowed(b"the first entry")),
                    id: Some(Cow::Borrowed(b"1")),
                    // Fri, 20 Jun 2003 09:00:00 GMT
                    pub_date: datetime(2003, 06, 20, 09, 00, 00, 00)
                        .to_zoned(tz::GMT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;
                        Box::slice(Box::new_in(*b"https://example.com/entry_1.mp3", &alloc)),
                        Box::slice(Box::new_in(*b"", &alloc))
                    ],
                },
                Entry {
                    title: None,
                    link: Some(Cow::Borrowed(b"https://example.com/entry_2")),
                    description: None,
                    id: Some(Cow::Borrowed(b"https://example.com/entry_2")),
                    pub_date: None,
                    enclosures: vec![in &alloc;],
                },
            ],
            &alloc,
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
                skip_days: SkipDays::default(),
                ttl: None,
            },
            [],
            &alloc::Dummy,
        )
    }

    #[test]
    fn test_rss_parser_2_0() -> Result<(), TestParserError<'static>> {
        let alloc = Bump::<Global>::try_new()?;

        test_parser::<_, Step, _>(
            include_str!("./sample-rss-2.xml"),
            Channel {
                title: Some(Cow::Borrowed(b"NASA Space Station News")),
                link: Some(Cow::Borrowed(b"http://www.nasa.gov/")),
                modify_date: Some(Replaceable {
                    // Fri, 21 Jul 2023 09:04 EDT
                    data: datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    replaceable: false,
                }),
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: None,
            },
            [
                Entry {
                    title: Some(Cow::Borrowed(b"Louisiana Students to Hear from NASA Astronauts Aboard Space Station")),
                    link: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/louisiana-students-to-hear-from-nasa-astronauts-aboard-space-station")),
                    description: Some(Cow::Borrowed(b"As part of the state's first Earth-to-space call, students from Louisiana will have an opportunity soon to hear from NASA astronauts aboard the International Space Station.")),
                    id: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/louisiana-students-to-hear-from-nasa-astronauts-aboard-space-station")),
                    // Fri, 21 Jul 2023 09:04 EDT
                    pub_date: datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;],
                },
                Entry {
                    title: None,
                    link: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-awards-integrated-mission-operations-contract-iii")),
                    description: Some(Cow::Borrowed(b"NASA has selected KBR Wyle Services, LLC, of Fulton, Maryland, to provide mission and flight crew operations support for the International Space Station and future human space exploration.")),
                    id: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-awards-integrated-mission-operations-contract-iii")),
                    // Thu, 20 Jul 2023 15:05 EDT
                    pub_date: datetime(2023, 07, 20, 15, 05, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"NASA Expands Options for Spacewalking, Moonwalking Suits")),
                    link: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-expands-options-for-spacewalking-moonwalking-suits-services")),
                    description: Some(Cow::Borrowed(b"NASA has awarded Axiom Space and Collins Aerospace task orders under existing contracts to advance spacewalking capabilities in low Earth orbit, as well as moonwalking services for Artemis missions.")),
                    id: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-expands-options-for-spacewalking-moonwalking-suits-services")),
                    // Mon, 10 Jul 2023 14:14 EDT
                    pub_date: datetime(2023, 07, 10, 14, 14, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;
                        Box::slice(Box::new_in(*b"http://www.nasa.gov/sites/default/files/styles/1x1_cardfeed/public/thumbnails/image/iss068e027836orig.jpg?itok=ucNUaaGx", &alloc)),
                    ],
                },
                Entry {
                    title: Some(Cow::Borrowed(b"NASA to Provide Coverage as Dragon Departs Station")),
                    link: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-to-provide-coverage-as-dragon-departs-station-with-science")),
                    description: Some(Cow::Borrowed(b"NASA is set to receive scientific research samples and hardware as a SpaceX Dragon cargo resupply spacecraft departs the International Space Station on Thursday, June 29.")),
                    id: Some(Cow::Borrowed(b"http://www.nasa.gov/press-release/nasa-to-provide-coverage-as-dragon-departs-station-with-science")),
                    // Tue, 20 May 2003 08:56:02 GMT
                    pub_date: datetime(2003, 05, 20, 08, 56, 02, 00)
                        .to_zoned(tz::GMT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"NASA Plans Coverage of Roscosmos Spacewalk Outside Space Station")),
                    link: Some(Cow::Borrowed(b"http://liftoff.msfc.nasa.gov/news/2003/news-laundry.asp")),
                    description: Some(Cow::Borrowed(b"Compared to earlier spacecraft, the International Space Station has many luxuries, but laundry facilities are not one of them.  Instead, astronauts have other options.")),
                    id: Some(Cow::Borrowed(b"http://liftoff.msfc.nasa.gov/2003/05/20.html#item570")),
                    // Mon, 26 Jun 2023 12:45 EDT
                    pub_date: datetime(2023, 06, 26, 12, 45, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp()
                        .into(),
                    enclosures: vec![in &alloc;
                        Box::slice(Box::new_in(*b"http://www.nasa.gov/sites/default/files/styles/1x1_cardfeed/public/thumbnails/image/spacex_dragon_june_29.jpg?itok=nIYlBLme", &alloc))
                    ]
                }
            ],
            &alloc,
        )
    }
}
