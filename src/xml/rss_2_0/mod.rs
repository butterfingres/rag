#[cfg(test)]
mod tests;

use {
    crate::{
        borrow::Cow,
        xml::{
            self, HandleElementInto, ParserError, Replaceable, ReplaceableHandler,
            Rfc2822Timestamp, TryFromRootError,
        },
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        events::{BytesStart, Event},
        reader::NsReader,
    },
    std::fmt::{self, Debug, Formatter},
};

#[derive(Default)]
pub enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
}

#[derive(Default)]
pub struct Channel<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
    link: Option<Cow<'src, [u8], &'alloc A>>,
    modify_date: Option<Replaceable<Rfc2822Timestamp>>,
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

impl<'alloc, 'src, A> xml::Parser<'alloc, 'src, A> for Step
where
    A: Allocator + ?Sized + 'alloc,
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
    fn handle_event(
        self,
        reader: &mut NsReader<&'src [u8]>,
        event: Event<'src>,
        state: &mut Channel<'alloc, 'src, A>,
        alloc: &'alloc A,
    ) -> Result<Self, ParserError> {
        match (self, event) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::InsideChannel)
            }
            (Step::InsideChannel, Event::End(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::OutsideChannel)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"title" => {
                Option::<Cow<'_, [u8], &A>>::handle_element_into(
                    &mut state.title,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"link" => {
                Option::<Cow<'_, [u8], &A>>::handle_element_into(
                    &mut state.link,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"pubDate" => {
                Option::<ReplaceableHandler<true, _>>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }
            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"lastBuildDate" => {
                Option::<ReplaceableHandler<false, _>>::handle_element_into(
                    &mut state.modify_date,
                    reader,
                    tag.name(),
                    alloc,
                )
                .map(|_| step)
            }

            (step, Event::Start(tag)) => {
                reader.read_to_end(tag.name())?;
                Ok(step)
            }

            (step, _) => Ok(step),
        }
    }
}
