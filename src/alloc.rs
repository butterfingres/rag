use {
    allocator_api2::alloc::{AllocError, Allocator},
    crossbeam_deque::{Injector, Steal},
    std::{alloc::Layout, ptr::NonNull, sync::LazyLock},
};

/// Allocator that never allocates.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "this allocator is only used in tests")
)]
#[derive(Debug)]
pub struct Dummy;

// SAFETY: This never allocates.
unsafe impl Allocator for Dummy {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }
    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}

type BumpSettings = bump_scope::settings::BumpSettings<
    // MIN_ALIGN
    1,
    // UP
    true,
    // GUARANTEED_ALLOCATED
    false,
    // CLAIMABLE
    false,
    // DEALLOCATES
    true,
    // SHRINKS
    true,
    // MINIMUM_CHUNK_SIZE
    512,
>;
pub type Bump = bump_scope::Bump<bump_scope::alloc::Global, BumpSettings>;

static ALLOCATOR_QUEUE: LazyLock<Injector<Bump>> = LazyLock::new(|| Injector::new());

pub fn with_bump<F, T>(f: F) -> T
where
    F: FnOnce(&mut Bump) -> T,
{
    let mut alloc = loop {
        match ALLOCATOR_QUEUE.steal() {
            Steal::Success(alloc) => break alloc,
            Steal::Empty => break Bump::unallocated(),
            Steal::Retry => {}
        }
    };

    let output = f(&mut alloc);

    alloc.reset();
    ALLOCATOR_QUEUE.push(alloc);

    output
}

#[cfg(test)]
mod tests {
    use {super::*, allocator_api2::alloc::LayoutError};

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
}
