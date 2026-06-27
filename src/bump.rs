use {allocator_api2::alloc::Global, bump_scope::Bump};

/// Create a bump allocator.
#[emacs::defun(user_ptr)]
fn new() -> Result<Bump<Global>, emacs::Error> {
    Bump::try_new().map_err(emacs::Error::new)
}

/// Reset the bump allocator.
#[emacs::defun]
fn reset(alloc: &mut Bump<Global>) -> Result<(), emacs::Error> {
    alloc.reset_to_start();
    Ok(())
}
