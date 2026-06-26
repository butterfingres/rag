use crate::sym;

/// Return the the buffer contents as a string.
///
/// This function is only available in lisp code when compling with
/// debug assertions as this is only intended to be used in tests.
///
/// If you need to get a buffer's string efficiently, use
/// `buffer-substring` because this function will copy the string.
#[cfg_attr(debug_assertions, emacs::defun(name = "-string"))]
fn string(env: &emacs::Env) -> Result<String, emacs::Error> {
    sym::fun::BUFFER_SUBSTRING_NO_PROPERTIES
        .call(
            env,
            (
                sym::fun::POINT_MIN.call(env, [])?,
                sym::fun::POINT_MAX.call(env, [])?,
            ),
        )?
        .into_rust()
}
