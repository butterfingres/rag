pub mod alloc;
pub mod borrow;
pub mod buffer;
mod fmt;
mod num;
mod sym;
pub mod tz;
pub mod xml;

emacs::plugin_is_GPL_compatible!();

#[emacs::module(name = "rag-core")]
fn init(_: &emacs::Env) -> Result<(), emacs::Error> {
    Ok(())
}
