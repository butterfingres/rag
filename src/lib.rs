#![allow(
    clippy::zero_prefixed_literal,
    reason = "dates and times look more clear with a prefixed zero"
)]

#[cfg(test)]
mod alloc;
mod borrow;
mod bump;
mod feed;
mod fmt;
mod num;
mod string;
mod sym;
mod tz;
mod xml;

use {
    crate::xml::{
        Parser,
        fmt::{atom, rdf, rss},
        get_header,
    },
    allocator_api2::alloc::Global,
    bump_scope::Bump,
    quick_xml::reader::NsReader,
    rem::IntoLisp,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

const ABI_VERSION: u32 = 2;

#[derive(Debug)]
struct IncompatibleAbiVersionError(u32);
impl Display for IncompatibleAbiVersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "expected abi version {}, found {}", ABI_VERSION, self.0)
    }
}
impl Error for IncompatibleAbiVersionError {}

rem::plugin_is_GPL_compatible!();

#[rem::module(name = "rag-core")]
fn init(env: &rem::Env) -> Result<(), rem::Error> {
    sym::fun::REQUIRE
        .try_bind(env)?
        .call(env, (&sym::val::RAG_LIB,))?;

    let version = sym::fun::SYMBOL_VALUE
        .try_bind(env)?
        .call(env, (&sym::val::RAG_ABI_VERSION,))?
        .into_rust::<u32>(env)?;
    if version != ABI_VERSION {
        return Err(rem::Error::from(IncompatibleAbiVersionError(version)));
    }

    env.lambda(&bump::New, None)?.fset("rag-core-bump-new")?;
    env.lambda(&bump::Reset, None)?
        .fset("rag-core-bump-reset")?;
    env.lambda(&feed::FetchP, None)?
        .fset("rag-core-feed-fetch-p")?;
    env.lambda(
        &ParseString,
        Some(
            c"Parse STRING into a `rag-feed'.

ALLOC should be a bump allocator created by `rag-core-bump-new'.
ENTRY-HANDLER is a function that will be called with `rag-entry'
objects.

The `rag-entry' objects when passed to ENTRY-HANDLER will not contain
a `rag-entry-feed-id' field, you will need to store the feed url by
capturing it in a closure.",
        ),
    )?
    .fset("rag-core-parse-string")?;

    Ok(())
}

#[derive(Debug)]
pub struct UnknownRootError;
impl Display for UnknownRootError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str("unknown root tag")
    }
}
impl Error for UnknownRootError {}

#[rem::defun]
fn parse_string<'e>(
    env: &'e rem::Env,
    string: rem::Value<'e>,
    alloc: &Bump<Global>,
    entry_handler: rem::Value<'e>,
) -> Result<rem::Value<'e>, rem::Error> {
    let string = string::from_lisp_in(env, string, alloc)?;
    let mut reader = NsReader::from_str(&string);
    let (version, root) = get_header(&mut reader)?;

    let parsers: [&dyn Parser<'_, '_, Bump<Global>>; 3] =
        [&atom::Parser, &rdf::Parser, &rss::Parser];

    for parser in parsers {
        if parser.try_recognize_root(&root, &reader, version)? {
            let feed = parser.handle_events(
                &mut reader,
                &mut |entry| {
                    let entry = entry.into_lisp(env)?;
                    entry_handler.call(env, (entry,))?;
                    Ok(())
                },
                version,
                alloc,
            )?;
            return feed.into_lisp(env);
        }
    }

    Err(rem::Error::from(UnknownRootError))
}
