pub mod alloc;
pub mod borrow;
pub mod bump;
mod fmt;
mod num;
mod sym;
pub mod tz;
pub mod xml;

emacs::plugin_is_GPL_compatible!();

use {
    crate::xml::{Parser, TryFromRootError, atom, get_header, rdf, rss},
    allocator_api2::alloc::Global,
    arrayvec::ArrayVec,
    bump_scope::Bump,
    emacs::IntoLisp,
    quick_xml::reader::NsReader,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

const ABI_VERSION: u32 = 0;

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
#[emacs::defun]
fn parse_string<'e>(
    env: &'e emacs::Env,
    string: String,
    alloc: &Bump<Global>,
    _entry_handler: emacs::Value<'e>,
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
                    let feed = parser.handle_events(&mut reader, |_cb| Ok(()), version, alloc)?;

                    let mut args = ArrayVec::<emacs::Value<'e>, { 6 * 2 }>::new();
                    if let Some(val) = feed.title {
                        let val = String::from_utf8_lossy(&val);
                        args.push(sym::key::TITLE.bind(env));
                        args.push(val.as_ref().into_lisp(env)?);
                    }

                    if let Some(val) = feed.link {
                        let val = String::from_utf8_lossy(&val);
                        args.push(sym::key::LINK.bind(env));
                        args.push(val.as_ref().into_lisp(env)?);
                    }

                    args.push(sym::key::SKIP_DAYS.bind(env));
                    args.push(feed.skip_days[0].into_lisp(env)?);

                    args.push(sym::key::SKIP_HOURS.bind(env));
                    args.push(feed.skip_hours[0].into_lisp(env)?);

                    if let Some(val) = feed.ttl {
                        args.push(sym::key::TTL.bind(env));
                        args.push(val.into_lisp(env)?);
                    }

                    if let Some(val) = feed.last_update {
                        args.push(sym::key::LAST_UPDATE.bind(env));
                        args.push(val.as_second().into_lisp(env)?);
                    }

                    let feed = sym::val::MAKE_RAG_FEED.call(env, args.as_ref())?;

                    return Ok(feed);
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
