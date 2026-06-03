pub mod feed;
pub mod rfc822;
pub mod split;
pub mod sym;
pub mod tz;
pub mod utf8;

emacs::plugin_is_GPL_compatible!();

#[emacs::module]
fn init(env: &emacs::Env) -> Result<(), emacs::Error> {
    sym::f::REQUIRE.call(env, (sym::v::RAG,))?;

    Ok(())
}
