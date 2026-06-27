pub mod alloc;
pub mod borrow;
mod fmt;
mod num;
mod sym;
pub mod tz;
pub mod xml;

emacs::plugin_is_GPL_compatible!();

use std::{
    error::Error,
    fmt::{Display, Formatter},
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
