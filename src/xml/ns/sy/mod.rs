//! Syndication module parser.
//!
//! See <https://web.resource.org/rss/1.0/modules/syndication/>.

use {
    crate::{
        num,
        xml::{ParserError, PartialFeed, ns::HandleStart, read_to_end},
    },
    allocator_api2::alloc::Allocator,
    jiff::Span,
    quick_xml::{XmlVersion, events::Event, reader::NsReader},
};

pub const NS: &[u8] = b"http://purl.org/rss/1.0/modules/syndication/";

pub struct Parser;
impl<'alloc, 'src, A> HandleStart<'alloc, 'src, PartialFeed<'alloc, 'src, A>, A> for Parser
where
    A: Allocator,
{
    fn handle_start(
        &self,
        reader: &mut NsReader<&'src [u8]>,
        start: Event<'src>,
        feed: &mut PartialFeed<'alloc, 'src, A>,
        _version: XmlVersion,
        alloc: &'alloc A,
    ) -> Result<(), ParserError> {
        match start {
            Event::Start(tag) if tag.local_name().as_ref() == b"updatePeriod" => {
                match read_to_end(reader, tag.name(), alloc)?.as_ref() {
                    b"hourly" => {
                        feed.period = Span::new().try_hours(1)?;
                    }
                    b"daily" => {
                        feed.period = Span::new().try_days(1)?;
                    }
                    b"weekly" => {
                        feed.period = Span::new().try_weeks(1)?;
                    }
                    b"monthly" => {
                        feed.period = Span::new().try_months(1)?;
                    }
                    b"yearly" => {
                        feed.period = Span::new().try_years(1)?;
                    }
                    _ => {}
                }
            }
            Event::Start(tag) if tag.local_name().as_ref() == b"updateFrequency" => {
                feed.frequency = Some(num::parse(read_to_end(reader, tag.name(), alloc)?)?);
            }

            Event::Start(tag) => {
                reader.read_to_end(tag.name())?;
            }

            _ => {}
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            alloc::Dummy,
            xml::{Feed, SkipDays, SkipHours, ns::tests::test_feed_parser},
        },
    };

    #[test]
    fn test_dc_parser_item() -> Result<(), ParserError> {
        let alloc = Dummy;
        test_feed_parser(
            &Parser,
            include_str!("./channel.xml"),
            Feed {
                title: None,
                link: None,
                skip_days: SkipDays::default(),
                skip_hours: SkipHours::default(),
                ttl: Span::new().try_hours(1)?,
                frequency: Some(20),
                last_update: None,
            },
            &alloc,
        )
    }
}
