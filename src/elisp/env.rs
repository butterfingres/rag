use {
    crate::elisp::sys::emacs_env_28,
    std::{marker::PhantomData, mem, ptr::NonNull},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env<'e> {
    ptr: NonNull<emacs_env_28>,
    _marker: PhantomData<&'e ()>,
}
impl Env<'_> {
    /// Create an `Env` object from a pointer.
    ///
    /// # Safety
    ///
    /// - The first field in the pointer must be the first field of
    /// [emacs_env_28] reporting the size of the struct that is safe
    /// to read and that portion must follow the abi of
    /// [emacs_env_28].
    pub unsafe fn try_from_ptr(env: NonNull<emacs_env_28>) -> Option<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE_FITS: &str = "the size should fit";

    #[test]
    fn env_try_from_ptr_too_small() {
        #[repr(C)]
        struct EmacsEnv {
            size: isize,
        }
        impl Default for EmacsEnv {
            fn default() -> Self {
                Self {
                    size: mem::size_of::<Self>().try_into().expect(SIZE_FITS),
                }
            }
        }

        let env = EmacsEnv::default();
        let ptr = NonNull::from_ref(&env).cast();
        // SAFETY: we correctly point out the size of the struct
        assert_eq!(unsafe { Env::try_from_ptr(ptr) }, None);
    }

    #[test]
    fn env_try_from_ptr() {
        let mut env = emacs_env_28::default();
        env.size = mem::size_of::<emacs_env_28>().try_into().expect(SIZE_FITS);
        let ptr = NonNull::from_ref(&env);
        // SAFETY: we correctly point out the size of the struct
        assert_eq!(
            unsafe { Env::try_from_ptr(ptr) },
            Some(Env {
                ptr,
                _marker: PhantomData,
            })
        );
    }
}
