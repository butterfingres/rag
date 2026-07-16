use {allocator_api2::alloc::Global, bump_scope::Bump};

#[rem::defun(user_ptr)]
fn new() -> Result<Bump<Global>, rem::Error> {
    Bump::try_new().map_err(rem::Error::from)
}

#[rem::defun]
fn reset(alloc: &mut Bump<Global>) -> Result<(), rem::Error> {
    alloc.reset_to_start();
    Ok(())
}
