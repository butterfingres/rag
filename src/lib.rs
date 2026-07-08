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
mod sym;
mod tz;
mod xml;

emacs::plugin_is_GPL_compatible!();

use {
    crate::xml::{
        Parser, TryFromRootError,
        fmt::{atom, rdf, rss},
        get_header,
    },
    allocator_api2::alloc::Global,
    bump_scope::Bump,
    emacs::IntoLisp,
    quick_xml::reader::NsReader,
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

#[emacs::module(name = "rag-core")]
fn init(env: &emacs::Env) -> Result<(), emacs::Error> {
    sym::fun::REQUIRE.call(env, (sym::val::RAG_LIB.bind(env),))?;

    let version = sym::fun::SYMBOL_VALUE
        .call(env, (sym::val::RAG_ABI_VERSION.bind(env),))?
        .into_rust::<u32>()?;
    if version != ABI_VERSION {
        return Err(emacs::Error::new(IncompatibleAbiVersionError(version)));
    }

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

/// Parse STRING into a `rag-feed'.
///
/// ALLOC should be a bump allocator created by `rag-core-bump-new'.
/// ENTRY-HANDLER is a function that will be called with `rag-entry'
/// objects.
///
/// The `rag-entry' objects when passed to ENTRY-HANDLER will not
/// contain a `rag-entry-feed-id' field, you will need to store the
/// feed url by capturing it in a closure.
#[emacs::defun]
fn parse_string<'e>(
    env: &'e emacs::Env,
    string: String,
    alloc: &Bump<Global>,
    entry_handler: emacs::Value<'e>,
) -> Result<emacs::Value<'e>, emacs::Error> {
    let mut reader = NsReader::from_str(&string);
    let (version, root) = get_header(&mut reader)?;

    macro_rules! try_parsers {
        ($root:expr, []) => {
            let _ = $root;
            return Err(emacs::Error::new(UnknownRootError));
        };
        ($root:expr, [$car:ty $(, $($cdr:ty),* $(,)?)?]) => {
            match <$car as Parser<'_, '_, &Bump<Global>>>::try_from_root($root, &reader, version) {
                Ok(parser) => {
                    let feed = parser.handle_events(&mut reader, |entry| {
                        let entry = entry.into_lisp(env)?;
                        entry_handler.call((entry,))?;
                        Ok(())
                    }, version, alloc)?;
                    return feed.into_lisp(env);
                }
                Err(TryFromRootError::UnknownRoot(root)) => {
                    try_parsers!(root, [$($($cdr),*)?]);
                }
                Err(TryFromRootError::Xml(e)) => {
                    return Err(emacs::Error::new(e));
                }
            }
        };
    }
    try_parsers!(root, [atom::Parser, rdf::Parser, rss::Parser]);
}
