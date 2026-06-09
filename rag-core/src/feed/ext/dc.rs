//! Dublin core support

use crate::{
    feed::{
        ext::RootExtension,
        fmt::{ParserError, PartialFeed},
    },
    utf8::Start,
};

pub struct Dc;
impl RootExtension for Dc {
    const NS: &str = "dc";

    fn handle_start<'a>(_start: Start<'a>, _feed: &mut PartialFeed<'a>) -> Result<(), ParserError> {
        Ok(())
    }
}
