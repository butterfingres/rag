use {
    allocator_api2::alloc::{AllocError, Allocator},
    bump_scope::Bump,
    std::{
        alloc::{Layout, LayoutError},
        ptr::NonNull,
        sync::LazyLock,
    },
};

/// Allocator that never allocates.
#[derive(Debug)]
pub struct Dummy;

// SAFETY: This never allocates.
unsafe impl Allocator for Dummy {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }
    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}

static CHANNEL: LazyLock<(
    crossbeam_channel::Sender<Bump>,
    crossbeam_channel::Receiver<Bump>,
)> = LazyLock::new(|| crossbeam_channel::bounded(16));

pub fn with_bump<F, T>(f: F) -> T
where
    F: FnOnce(&mut Bump) -> T,
{
    let (tx, rx) = &*CHANNEL;
    let mut bump = rx.try_recv().unwrap_or_else(|_| Bump::try_new().unwrap());
    let val = f(&mut bump);
    bump.reset_to_start();
    let _ = tx.try_send(bump);
    val
}

#[test]
fn failing_allocator() -> Result<(), LayoutError> {
    [
        Layout::new::<u8>(),
        Layout::new::<u128>(),
        Layout::array::<u8>(512)?,
    ]
    .into_iter()
    .for_each(|layout| {
        assert_eq!(Dummy.allocate(layout), Err(AllocError));
    });
    Ok(())
}
