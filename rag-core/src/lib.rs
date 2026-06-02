pub mod feed;
pub mod sym;

emacs::plugin_is_GPL_compatible!();

#[emacs::module]
fn init(env: &emacs::Env) -> Result<(), emacs::Error> {
    sym::f::REQUIRE.call(env, (sym::v::RAG,))?;

    Ok(())
}
