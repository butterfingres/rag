use {
    crate::{
        borrow::Cow,
        xml::{self, HandleElement, ParserError, TryFromRootError},
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

pub struct Channel<'alloc, 'src, A>
where
    A: Allocator + ?Sized,
{
    title: Option<Cow<'src, [u8], &'alloc A>>,
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
                Option::<_>::handle_element(&mut state.title, reader, tag.name(), alloc)
                    .map(|_| step)
            }
            (step, _) => Ok(step),
        }
    }
}
