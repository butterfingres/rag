use {
    crate::elisp::sys::emacs_env_28,
    std::{marker::PhantomData, mem, ptr::NonNull},
};

#[derive(Clone, Copy)]
pub struct Env<'e> {
    ptr: NonNull<emacs_env_28>,
    _marker: PhantomData<&'e ()>,
}
impl Env<'_> {
    unsafe fn try_from_env(env: NonNull<emacs_env_28>) -> Option<Self> {
        // SAFETY: there would be no point passing a struct that you
        // cannot read the size from.
        usize::try_from(unsafe { (*env.as_ptr()).size })
            .ok()
            .filter(|found_size| *found_size >= mem::size_of::<emacs_env_28>())?;

        Some(Self {
            ptr: env,
            _marker: PhantomData,
        })
    }
}
