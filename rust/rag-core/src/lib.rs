pub mod io;
mod sym;

emacs::plugin_is_GPL_compatible!();

#[emacs::module(name = "rag-core")]
fn init(_: &emacs::Env) -> Result<(), emacs::Error> {
    Ok(())
}
