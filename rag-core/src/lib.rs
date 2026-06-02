pub mod feed;
pub mod sym;

emacs::plugin_is_GPL_compatible!();

#[emacs::module]
fn init(_: &emacs::Env) -> Result<(), emacs::Error> {
    Ok(())
}
