use {
    crate::{
        borrow::Cow,
        xml::{self, HandleElement, ParserError},
    },
    allocator_api2::alloc::Allocator,
    quick_xml::{
        events::{BytesStart, Event},
        reader::NsReader,
    },
};

#[derive(Default)]
pub enum Step {
    #[default]
    OutsideChannel,
    InsideChannel,
}

pub struct Channel<'a, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'a, [u8], &'a A>>,
}

impl<'a, A> xml::Parser<'a, A> for Step
where
    A: Allocator + ?Sized + 'a,
{
    type State = Channel<'a, A>;
    fn try_from_root(tag: BytesStart<'a>) -> Result<Self, BytesStart<'a>> {
        if tag.name().0 == b"rss" {
            Ok(Self::OutsideChannel)
        } else {
            Err(tag)
        }
    }
    fn handle_event(
        self,
        reader: &mut NsReader<&'a [u8]>,
        event: Event<'a>,
        state: &mut Channel<'a, A>,
        alloc: &'a A,
    ) -> Result<Self, ParserError> {
        match (self, event) {
            (Step::OutsideChannel, Event::Start(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::InsideChannel)
            }
            (Step::InsideChannel, Event::End(tag)) if tag.name().0 == b"channel" => {
                Ok(Self::OutsideChannel)
            }

            (step @ Step::InsideChannel, Event::Start(tag)) if tag.name().0 == b"title" => {
                // state.title = Option::han;
                // Option::<>::handle_element(&mut state.title, reader, tag.name(), alloc)?;
                // Option::<Cow<'a, [u8], &'a A>>::handle_element(
                //     &mut state.title,
                //     reader,
                //     tag.name(),
                //     alloc,
                // )?;
                Ok(step)
            }
            (step, _) => Ok(step),
        }
    }
}
