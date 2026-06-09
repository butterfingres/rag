pub mod dc;

use crate::{
    feed::fmt::{ParserError, PartialFeed},
    utf8::Start,
};

pub trait RootExtension {
    const NS: &str;

    fn handle_start<'a>(_: Start<'a>, _: &mut PartialFeed<'a>) -> Result<(), ParserError>;
}
