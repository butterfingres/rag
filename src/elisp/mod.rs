pub mod sys;

use std::ffi::c_int;

#[unsafe(no_mangle)]
static plugin_is_GPL_compatible: c_int = 0;
