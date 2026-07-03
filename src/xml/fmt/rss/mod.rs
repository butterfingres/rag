use {
    crate::{
        num,
        xml::{
            self, Entry, ParserError, PartialEntry, PartialFeed, Replaceable, TryFromRootError,
            get_attribute_when, ns,
            parser::{Content, ParseTagInto, TagParser, rfc2822_timestamp},
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
        XmlVersion,
        events::{BytesStart, Event},
        name::{Namespace, QName, ResolveResult},
        reader::NsReader,
    },
    std::marker::PhantomData,
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
impl<'alloc, 'src, T, A> ParseTagInto<'alloc, 'src, A, BitArray<T::View, T::Order>>
    for RssSkipHandler<T>
where
    T: RssSkip,
    A: Allocator,
{
    fn parse_tag_into(
        bitvec: &mut BitArray<T::View, T::Order>,
        reader: &mut NsReader<&'src [u8]>,
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

fn handle_enclosure<'alloc, 'src, A>(
    item: &mut PartialEntry<'alloc, 'src, A>,
    enclosure: BytesStart<'src>,
    version: XmlVersion,
    alloc: &'alloc A,
) -> Result<(), ParserError>
where
    A: Allocator,
{
    if let Some(enclosure) = get_attribute_when(
        &enclosure,
        |_| Ok(true),
        |attr| attr.key.0 == b"url",
        version,
        alloc,
    )? {
        item.enclosures.push(enclosure);
    }

    Ok(())
}

struct RssItem;
impl<'alloc, 'src, F, T, A> ParseTagInto<'alloc, 'src, A, F> for RssItem
where
    F: FnMut(Entry<'alloc, 'src, A>) -> T,
    T: Into<Result<(), ParserError>>,
    A: Allocator + 'alloc,
{
    fn parse_tag_into(
        cb: &mut F,
        reader: &mut NsReader<&'src [u8]>,
        name: QName<'_>,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        let mut item = PartialEntry::new_in(alloc);
        loop {
            match reader.read_resolved_event()? {
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"title" => {
                    item.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;
                }
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"link" => {
                    item.link.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"description" => {
                    item.content.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"guid" => {
                    let mut is_permalink = None;
                    for attr in tag.attributes() {
                        let attr = attr?;
                        if attr.key.0 == b"isPermalink" {
                            is_permalink = Some(attr.value.as_ref() == b"true");
                            break;
                        }
                    }

                    let link = Content.parse_tag(reader, tag.name(), version, alloc)?;
                    if is_permalink.unwrap_or(true)
                        && let Replaceable {
                            replaceable: true, ..
                        } = item.link
                    {
                        item.link = Replaceable {
                            data: Some(link.clone()),
                            replaceable: false,
                        };
                    }
                    item.id = Replaceable::new_irreplaceable(Some(link));
                }
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"pubDate" => {
                    item.updated.try_replace_or_skip(
                        Content
                            .flat_map(rfc2822_timestamp)
                            .map(Some)
                            .map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;
                }
                (ResolveResult::Unbound, Event::Start(tag)) if tag.name().0 == b"enclosure" => {
                    reader.read_to_end(tag.name())?;
                    handle_enclosure(&mut item, tag, version, alloc)?;
                }
                (ResolveResult::Unbound, Event::Empty(tag)) if tag.name().0 == b"enclosure" => {
                    handle_enclosure(&mut item, tag, version, alloc)?;
                }

                (
                    ResolveResult::Bound(Namespace(ns)),
                    event @ (Event::Start(_) | Event::Empty(_)),
                ) if let Some(handler) = ns::item_handler(ns) => {
                    handler.handle_start(reader, event, &mut item, version, alloc)?;
                }
                (_, Event::Start(tag)) => {
                    reader.read_to_end(tag.name())?;
                }

                (_, Event::End(tag)) if tag.name() == name => {
                    cb(item.into()).into()?;
                    return Ok(());
                }
                (_, Event::Eof) => return Err(ParserError::UNCLOSED_TAG),

                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub enum Parser {
    #[default]
    OutsideChannel,
    InsideChannel,
}
impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for Parser
where
    A: Allocator + 'alloc,
{
    fn try_from_root(
        root: BytesStart<'src>,
        reader: &NsReader<&'src [u8]>,
        version: XmlVersion,
    ) -> Result<Self, TryFromRootError<'src>> {
        if let (ResolveResult::Unbound, name) = reader.resolver().resolve_element(root.name())
            && name.as_ref() == b"rss"
            && {
                let mut found = false;
                for attr in root.attributes() {
                    let attr = attr?;
                    if let (ResolveResult::Unbound, name) =
                        reader.resolver().resolve_attribute(attr.key)
                        && name.as_ref() == b"version"
                        && matches!(
                            attr.normalized_value(version)?.as_ref(),
                            "0.91" | "0.92" | "2.0"
                        )
                    {
                        found = true;
                        break;
                    }
                }
                found
            }
        {
            Ok(Self::OutsideChannel)
        } else {
            Err(TryFromRootError::UnknownRoot(root))
        }
    }
    fn handle_event<F>(
        self,
        reader: &mut NsReader<&'src [u8]>,
        event: Event<'src>,
        state: &mut PartialFeed<'alloc, 'src, A>,
        mut cb: F,
        version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<Self, ParserError>
    where
        F: FnMut(Entry<'alloc, 'src, A>) -> Result<(), ParserError>,
    {
        match &event {
            Event::Start(tag) => match (self, reader.resolver().resolve_element(tag.name())) {
                (Parser::OutsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"channel" =>
                {
                    Ok(Parser::InsideChannel)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"title" =>
                {
                    state.title.try_replace_with(|| {
                        Content
                            .map(Some)
                            .map(Replaceable::new_irreplaceable)
                            .parse_tag(reader, tag.name(), version, alloc)
                    })?;

                    Ok(step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"link" =>
                {
                    state.link.try_replace_or_skip(
                        Content.map(Some).map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;

                    Ok(step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"pubDate" =>
                {
                    state.last_update.try_replace_or_skip(
                        Content
                            .flat_map(rfc2822_timestamp)
                            .map(Some)
                            .map(Replaceable::new_replaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;

                    Ok(step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"lastBuildDate" =>
                {
                    state.last_update.try_replace_or_skip(
                        Content
                            .flat_map(rfc2822_timestamp)
                            .map(Some)
                            .map(Replaceable::new_irreplaceable),
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )?;

                    Ok(step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"skipHours" =>
                {
                    RssSkipHandler::<RssSkipHour>::parse_tag_into(
                        &mut state.skip_hours,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )
                    .map(|_| step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"skipDays" =>
                {
                    RssSkipHandler::<RssSkipDay>::parse_tag_into(
                        &mut state.skip_days,
                        reader,
                        tag.name(),
                        version,
                        alloc,
                    )
                    .map(|_| step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"ttl" =>
                {
                    state.ttl = Content
                        .flat_map(|val| num::parse(val).map_err(ParserError::ParseInt))
                        .map(Some)
                        .parse_tag(reader, tag.name(), version, alloc)?;

                    Ok(step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Unbound, name))
                    if name.as_ref() == b"item" =>
                {
                    RssItem::parse_tag_into(&mut cb, reader, tag.name(), version, alloc)
                        .map(|_| step)
                }
                (step @ Parser::InsideChannel, (ResolveResult::Bound(Namespace(ns)), _))
                    if let Some(handler) = ns::feed_handler(ns) =>
                {
                    handler
                        .handle_start(reader, event, state, version, alloc)
                        .map(|_| step)
                }
                (step, _) => {
                    reader.read_to_end(tag.name())?;
                    Ok(step)
                }
            },
            Event::Empty(tag)
                if let (ResolveResult::Bound(Namespace(ns)), _) =
                    reader.resolver().resolve_element(tag.name())
                    && let Some(handler) = ns::feed_handler(ns) =>
            {
                handler
                    .handle_start(reader, event, state, version, alloc)
                    .map(|_| self)
            }
            Event::End(tag)
                if let (ResolveResult::Unbound, name) =
                    reader.resolver().resolve_element(tag.name())
                    && let Parser::InsideChannel = self
                    && name.as_ref() == b"channel" =>
            {
                Ok(Self::OutsideChannel)
            }

            _ => Ok(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc::{self, with_bump},
            borrow::Cow,
            tz,
            xml::{
                Feed, SkipDays, SkipHours,
                tests::{TestParserError, test_parser},
            },
        },
        allocator_api2::{boxed::Box, vec},
        jiff::{Span, civil::datetime, tz::TimeZone},
    };

    #[test]
    fn test_rss_parser_all() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
                include_str!("./all.xml"),
                Feed {
                    title: Some(Cow::Borrowed(b"example feed")),
                    link: Some(Cow::Borrowed(b"https://example.com/rss")),
                    // Fri, 21 Jul 2023 09:04 EDT
                    last_update: Some(
                        datetime(2023, 07, 21, 09, 04, 00, 00)
                            .to_zoned(tz::EDT)?
                            .timestamp(),
                    ),
                    skip_hours: SkipHours::new([0b1110]),
                    skip_days: SkipDays::new([0b0111_1111]),
                    ttl: Span::new().try_minutes(30)?,
                    frequency: None,
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
        })
    }

    #[test]
    fn test_rss_parser_sample_v0_91() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
            include_str!("./sample-rss-091.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"WriteTheWeb")),
                link: Some(Cow::Borrowed(b"http://writetheweb.com")),
                last_update: None,
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: Span::new(),
                frequency: None,
            },
            [
                Entry {
                    title: Some(Cow::Borrowed(b"Giving the world a pluggable Gnutella")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=24")),
                    description: Some(Cow::Borrowed(b"WorldOS is a framework on which to build programs that work like Freenet or Gnutella -allowing distributed applications using peer-to-peer routing.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc;]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"Syndication discussions hot up")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=23")),
                    description: Some(Cow::Borrowed(b"After a period of dormancy, the Syndication mailing list has become active again, with contributions from leaders in traditional media and Web syndication.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc;]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"Personal web server integrates file sharing and messaging")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=22")),
                    description: Some(Cow::Borrowed(b"The Magi Project is an innovative project to create a combined personal web server and messaging system that enables the sharing and synchronization of information across desktop, laptop and palmtop devices.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"Syndication and Metadata")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=21")),
                    description: Some(Cow::Borrowed(b"RSS is probably the best known metadata format around. RDF is probably one of the least understood. In this essay, published on my O'Reilly Network weblog, I argue that the next generation of RSS should be based on RDF.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"UK bloggers get organised")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=20")),
                    description: Some(Cow::Borrowed(b"Looks like the weblogs scene is gathering pace beyond the shores of the US. There's now a UK-specific page on weblogs.com, and a mailing list at egroups.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc]
                },
                Entry {
                    title: Some(Cow::Borrowed(b"Yournamehere.com more important than anything")),
                    link: Some(Cow::Borrowed(b"http://writetheweb.com/read.php?item=19")),
                    description: Some(Cow::Borrowed(b"Whatever you're publishing on the web, your site name is the most valuable asset you have, according to Carl Steadman.")),
                    id: None,
                    pub_date: None,
                    enclosures: vec![in &alloc]
                },
            ],
            &alloc,
        )
        })
    }

    #[test]
    fn test_rss_parser_sample_v0_92() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
                include_str!("./sample-rss-092.xml"),
                Feed {
                    title: Some(Cow::Borrowed(b"Winnemac Daily News")),
                    link: Some(Cow::Borrowed(b"https://winnemac.example.com/")),
                    // Fri, 13 Apr 2001 09:03:49 GMT
                    last_update: Some(
                        datetime(2001, 04, 13, 09, 03, 49, 00)
                            .to_zoned(tz::GMT)?
                            .timestamp()
                    ),
                    skip_hours: SkipHours::default(),
                    skip_days: SkipDays::default(),
                    ttl: Span::new(),
                    frequency: None,
                },
                [
                    Entry {
                        title: Some(Cow::Borrowed(b"Cats and Dogs Form Unlikely Friendship")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/151")),
                        description: Some(Cow::Borrowed(b"In a heartwarming turn of events, a cat and a dog were spotted playing together in the park, proving that friendships can transcend species.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;]
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Local Artist\'s Painting Sells for Record Price")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/150")),
                        description: Some(Cow::Borrowed(b"A painting by a local artist recently sold at an auction for a staggering amount, setting a new record in the art world.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;]
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"New Movie Breaks Box Office Records")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/149")),
                        description: Some(Cow::Borrowed(b"The latest blockbuster movie has shattered box office records, becoming the highest-grossing film of all time. Moviegoers can\'t get enough of it.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc; Box::slice(Box::new_in(*b"https://winnemac.example.com/audio/movienews.mp3", &alloc))]
                    },
                    Entry {
                        title: None,
                        link: None,
                        description: Some(Cow::Borrowed(b"Our website will be undergoing scheduled maintenance from 2 a.m. to 6 a.m. tomorrow, as we revamp our servers to bring you an even better online news experience.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc]
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"World\'s Largest Ice Cream Sundae Created")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/148")),
                        description: Some(Cow::Borrowed(b"A team of chefs constructed the world\'s largest ice cream sundae, complete with a variety of toppings and flavors. It\'s a sight to behold.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc]
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Scientists Discover New Species in Amazon Rainforest")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/147")),
                        description: Some(Cow::Borrowed(b"An expedition into the Amazon rainforest led to the discovery of a new species of colorful birds, captivating the scientific community.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc; Box::slice(Box::new_in(*b"https://winnemac.example.com/audio/sciencenews.mp3", &alloc))]
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"World's Longest Bridge Opens to the Public")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/146")),
                        description: Some(Cow::Borrowed(b"A groundbreaking engineering marvel, the world's longest bridge, has finally opened, connecting two continents and easing transportation.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Scientists Develop Cure for Common Cold")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/145")),
                        description: Some(Cow::Borrowed(b"After years of research, scientists have unveiled a groundbreaking cure for the common cold, bringing relief to millions of people.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Robotics Competition Sparks Innovation")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/144")),
                        description: Some(Cow::Borrowed(b"Young minds showcase their creativity at a robotics competition, presenting innovative solutions to real-world challenges.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Ancient City Unearthed in the Desert")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/143")),
                        description: Some(Cow::Borrowed(b"Archaeologists make a historic discovery as they uncover the ruins of an ancient city buried deep in the desert sands.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"International Space Station Welcomes New Crew")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/142")),
                        description: Some(Cow::Borrowed(b"The International Space Station receives a fresh crew of astronauts, continuing scientific research and international cooperation in space.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Magical Forest Enchants Visitors")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/141")),
                        description: Some(Cow::Borrowed(b"A mystical forest with glowing plants and ethereal creatures captivates the imagination of visitors, drawing them into a magical realm.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Record-Breaking Heatwave Hits the Nation")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/140")),
                        description: Some(Cow::Borrowed(b"A scorching heatwave sweeps across the nation, setting new temperature records and prompting people to find creative ways to stay cool.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"Lost Treasure Found in Sunken Ship")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/139")),
                        description: Some(Cow::Borrowed(b"Divers stumble upon a sunken pirate ship and recover a long-lost treasure chest, sparking excitement among history enthusiasts.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"World's Largest Ferris Wheel Opens to Visitors")),
                        link: Some(Cow::Borrowed(b"https://winnemac.example.com/story/138")),
                        description: Some(Cow::Borrowed(b"Foodies rejoice as the city's famous food festival kicks off, offering a diverse range of mouthwatering cuisines from around the world.")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc],
                    },
                ],
                &alloc,
                    )
        })
    }

    #[test]
    fn test_rss_parser_sample_v2() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
                include_str!("./sample-rss-2.xml"),
                Feed {
                    title: Some(Cow::Borrowed(b"NASA Space Station News")),
                    link: Some(Cow::Borrowed(b"http://www.nasa.gov/")),
                    // Fri, 21 Jul 2023 09:04 EDT
                    last_update: Some(
                        datetime(2023, 07, 21, 09, 04, 00, 00)
                            .to_zoned(tz::EDT)?
                            .timestamp()
                    ),
                    skip_hours: SkipHours::default(),
                    skip_days: SkipDays::default(),
                    ttl: Span::new(),
                    frequency: None,
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
        })
    }

    #[test]
    fn test_rss_parser_ns() -> Result<(), TestParserError<'static>> {
        with_bump(|alloc| {
            test_parser::<_, Parser, _>(
                include_str!("./ns.xml"),
                Feed {
                    title: Some(Cow::Borrowed(b"dc title")),
                    link: None,
                    // 2026-07-03
                    last_update: Some(
                        datetime(2026, 07, 03, 00, 00, 00, 00)
                            .to_zoned(TimeZone::UTC)?
                            .timestamp(),
                    ),
                    skip_hours: SkipHours::default(),
                    skip_days: SkipDays::default(),
                    ttl: Span::new().try_hours(1)?,
                    frequency: Some(2),
                },
                [
                    Entry {
                        title: Some(Cow::Borrowed(b"dublin core entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"dublin core entry description")),
                        id: Some(Cow::Borrowed(b"1")),
                        // 2026-07-03
                        pub_date: Some(
                            datetime(2026, 07, 03, 00, 00, 00, 00)
                                .to_zoned(TimeZone::UTC)?
                                .timestamp(),
                        ),
                        enclosures: vec![in &alloc;],
                    },
                    Entry {
                        title: None,
                        link: None,
                        description: Some(Cow::Borrowed(b"content description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"media entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"media description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;
                                         Box::slice(Box::new_in(*b"https://example.com/media.mp3", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media.mp4", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media.torrent", &alloc)),
                        ],
                    },
                    Entry {
                        title: Some(Cow::Borrowed(b"media group entry")),
                        link: None,
                        description: Some(Cow::Borrowed(b"media group description")),
                        id: None,
                        pub_date: None,
                        enclosures: vec![in &alloc;
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.mp3", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.mp4", &alloc)),
                                         Box::slice(Box::new_in(*b"https://example.com/media_group.torrent", &alloc)),
                        ],
                    },
                ],
                &alloc,
            )
        })
    }

    #[test]
    fn test_rss_parser_alt() -> Result<(), TestParserError<'static>> {
        test_parser::<_, Parser, _>(
            include_str!("./alt.xml"),
            Feed {
                title: Some(Cow::Borrowed(b"example feed")),
                link: Some(Cow::Borrowed(b"https://example.com/rss")),
                // Fri, 21 Jul 2023 09:04 EDT
                last_update: Some(
                    datetime(2023, 07, 21, 09, 04, 00, 00)
                        .to_zoned(tz::EDT)?
                        .timestamp(),
                ),
                skip_hours: SkipHours::default(),
                skip_days: SkipDays::default(),
                ttl: Span::new(),
                frequency: None,
            },
            [],
            &alloc::Dummy,
        )
    }
}
