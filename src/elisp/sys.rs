#![allow(dead_code, reason = "not all the bindings here may be used")]
#![expect(
    clippy::enum_variant_names,
    non_camel_case_types,
    non_upper_case_globals,
    reason = "this module attempts to faithfully recreate the emacs-module.h type definitions"
)]

use {
    libc::timespec,
    std::{
        ffi::{c_char, c_double, c_int, c_void},
        ptr::NonNull,
    },
};

pub type ptrdiff_t = isize;
pub type intmax_t = isize;

pub type emacs_env = emacs_env_28;
pub type emacs_value = *mut c_void;

pub const emacs_variadic_function: c_int = -2;

#[repr(C)]
pub struct emacs_runtime {
    pub size: ptrdiff_t,
    pub private_members: *mut c_void,
    pub get_environment:
        Option<unsafe extern "C" fn(runtime: NonNull<emacs_runtime>) -> *mut emacs_env>,
}

pub type emacs_function = Option<
    unsafe extern "C" fn(
        env: NonNull<emacs_env>,
        nargs: ptrdiff_t,
        args: *mut emacs_value,
        data: *mut c_void,
    ),
>;
pub type emacs_finalizer = Option<unsafe extern "C" fn(data: *mut c_void)>;

#[repr(C)]
pub enum emacs_funcall_exit {
    emacs_funcall_exit_return = 0,
    emacs_funcall_exit_signal = 1,
    emacs_funcall_exit_throw = 2,
}

#[repr(C)]
pub enum emacs_process_input_result {
    emacs_process_input_continue = 0,
    emacs_process_input_quit = 1,
}

pub type emacs_limb_t = usize;
pub const EMACS_LIMB_MAX: emacs_limb_t = usize::MAX;

macro_rules! emacs_env_25 {
    ($(#[$attr:meta])*
     $vis:vis struct $ident:ident {
        $($f_vis:vis $f_ident:ident: $f_ty:ty,)* $(,)?
    }) => {
        $(#[$attr])*
        $vis struct $ident {
            pub size: ptrdiff_t,
            pub private_members: *mut c_void,

            pub make_global_ref: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, value: emacs_value)>,
            pub free_global_ref: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, global_value: emacs_value)>,

            pub non_local_exit_check: Option<unsafe extern "C" fn(env: NonNull<emacs_env>) -> emacs_funcall_exit>,
            pub non_local_exit_clear: Option<unsafe extern "C" fn(env: NonNull<emacs_env>)>,
            pub non_local_exit_get: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    symbol: NonNull<emacs_value>,
                    dataa: NonNull<emacs_value>,
                ) -> emacs_funcall_exit,
            >,
            pub non_local_exit_signal: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, symbol: emacs_value, data: emacs_value)>,
            pub non_local_exit_throw: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, tag: emacs_value, value: emacs_value)>,

            pub make_function: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    min_arity: ptrdiff_t,
                    max_arity: ptrdiff_t,
                    func: emacs_function,
                    docstring: *const c_char,
                    data: *mut c_void,
                ),
            >,
            pub funcall: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    func: emacs_value,
                    nargs: ptrdiff_t,
                    args: *mut emacs_value,
                ) -> emacs_value,
            >,
            pub intern: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, name: *const c_char) -> emacs_value>,

            pub type_of: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> emacs_value>,
            pub is_not_nil: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> bool>,
            pub eq: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, a: emacs_value, b: emacs_value) -> bool>,
            pub extract_integer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> intmax_t>,
            pub make_integer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, n: intmax_t) -> emacs_value>,
            pub extract_float: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> c_double>,
            pub make_float: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, n: c_double) -> emacs_value>,
            pub copy_string_contents: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    value: emacs_value,
                    buf: *mut c_char,
                    len: NonNull<ptrdiff_t>,
                ) -> bool,
            >,
            pub make_string: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    // should be NonNull
                    str: *const c_char,
                    len: ptrdiff_t,
                ) -> emacs_value,
            >,
            pub make_user_ptr: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, fin: emacs_finalizer, ptr: *mut c_void)>,
            pub get_user_ptr: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> *mut c_void>,
            pub set_user_ptr: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value, ptr: *mut c_void)>,
            pub get_user_finalizer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, uptr: emacs_value) -> emacs_finalizer>,
            pub set_user_finalizer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value, fin: emacs_finalizer)>,

            pub vec_get: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    vector: emacs_value,
                    index: ptrdiff_t,
                ) -> emacs_value,
            >,
            pub vec_set: Option<
                unsafe extern "C" fn(
                    env: NonNull<emacs_env>,
                    vector: emacs_value,
                    index: ptrdiff_t,
                    value: emacs_value,
                ),
            >,
            pub vec_size: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, vector: emacs_value) -> ptrdiff_t>,

            $($f_vis $f_ident: $f_ty,)*
        }
    }
}

emacs_env_25! {
    #[repr(C)]
    pub struct emacs_env_25 {}
}

macro_rules! emacs_env_26 {
    ($(#[$attr:meta])*
     $vis:vis struct $ident:ident {
        $($f_vis:vis $f_ident:ident: $f_ty:ty,)* $(,)?
     }) => {
        emacs_env_25! {
            $(#[$attr])*
            $vis struct $ident {
                pub should_quit: Option<unsafe extern "C" fn(env: NonNull<emacs_env>) -> bool>,

                $($f_vis $f_ident: $f_ty,)*
            }
        }
    }
}
emacs_env_26! {
    #[repr(C)]
    pub struct emacs_env_26 {}
}

macro_rules! emacs_env_27 {
    ($(#[$attr:meta])*
     $vis:vis struct $ident:ident {
        $($f_vis:vis $f_ident:ident: $f_ty:ty,)* $(,)?
     }) => {
        emacs_env_26! {
            $(#[$attr])*
            $vis struct $ident {
                pub process_input: Option<unsafe extern "C" fn(env: NonNull<emacs_env>) -> emacs_process_input_result>,

                pub extract_time: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> timespec>,
                pub make_time: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, time: timespec) -> emacs_value>,
                pub extract_big_integer: Option<
                    unsafe extern "C" fn(
                        env: NonNull<emacs_env>,
                        arg: emacs_value,
                        sign: *mut c_int,
                        count: *mut ptrdiff_t,
                        magnitude: *mut emacs_limb_t,
                    ),
                >,
                pub make_big_integer: Option<
                    unsafe extern "C" fn(
                        env: NonNull<emacs_env>,
                        sign: c_int,
                        count: ptrdiff_t,
                        magnitude: *const emacs_limb_t,
                    ),
                >,

                $($f_vis $f_ident: $f_ty,)*
            }
        }
    }
}
emacs_env_27! {
    #[repr(C)]
    pub struct emacs_env_27 {}
}

macro_rules! emacs_env_28 {
    ($(#[$attr:meta])*
     $vis:vis struct $ident:ident {
        $($f_vis:vis $f_ident:ident: $f_ty:ty,)* $(,)?
     }) => {
        emacs_env_27! {
            $(#[$attr])*
            $vis struct $ident {
                pub get_function_finalizer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value) -> emacs_finalizer>,
                pub set_function_finalizer: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, arg: emacs_value, fin: emacs_finalizer)>,

                pub open_channel: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, pipe_process: emacs_value) -> c_int>,

                pub make_interactive: Option<unsafe extern "C" fn(env: NonNull<emacs_env>, function: emacs_value, spec: emacs_value)>,
                pub make_unibyte_string: Option<
                    unsafe extern "C" fn(
                        env: NonNull<emacs_env>,
                        str: *const c_char,
                        len: ptrdiff_t,
                    ) -> emacs_value,
                >,

                $($f_vis $f_ident: $f_ty,)*
            }
        }
    }
}
emacs_env_28! {
    #[derive(Default)]
    #[repr(C)]
    pub struct emacs_env_28 {}
}

unsafe extern "C" {
    pub fn emacs_module_init(runtime: NonNull<emacs_runtime>) -> c_int;
}
