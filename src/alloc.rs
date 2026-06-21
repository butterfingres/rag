use {
    allocator_api2::alloc::{AllocError, Allocator},
    std::{alloc::Layout, ptr::NonNull},
};

/// Allocator that never allocates.
#[derive(Debug)]
pub struct DummyAllocator;

// SAFETY: This never allocates.
unsafe impl Allocator for DummyAllocator {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }
    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}

#[cfg(test)]
mod tests {
    use {super::*, std::alloc::LayoutError};

    #[test]
    fn failing_allocator() -> Result<(), LayoutError> {
        [
            Layout::new::<u8>(),
            Layout::new::<u128>(),
            Layout::array::<u8>(512)?,
        ]
        .into_iter()
        .for_each(|layout| {
            assert_eq!(DummyAllocator.allocate(layout), Err(AllocError));
        });
        Ok(())
    }
}
