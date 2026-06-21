use {
    allocator_api2::alloc::{AllocError, Allocator},
    std::{alloc::Layout, cell::Cell, ptr::NonNull},
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

/// An allocator that tracks whether an allocation has occurred.
pub struct Tracking<A>
where
    A: Allocator,
{
    alloc: A,
    allocated: Cell<bool>,
}
impl<A> From<A> for Tracking<A>
where
    A: Allocator,
{
    fn from(alloc: A) -> Self {
        Self {
            alloc,
            allocated: Cell::new(false),
        }
    }
}
// SAFETY: we rely on the safety of the underlying allocator
unsafe impl<A> Allocator for Tracking<A>
where
    A: Allocator,
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc.allocate(layout).inspect(|_| {
            self.allocated.set(true);
        })
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.alloc.deallocate(ptr, layout) }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        allocator_api2::{alloc::Global, vec::Vec},
        std::alloc::LayoutError,
    };

    fn must_allocate<A, F>(alloc: A, f: F)
    where
        A: Allocator,
        F: FnOnce(&Tracking<A>),
    {
        let alloc = Tracking::from(alloc);
        f(&alloc);
        assert!(alloc.allocated.get());
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

    #[test]
    fn tracking_allocator() {
        let alloc = Tracking::from(Dummy);
        Vec::<(), _>::new_in(&alloc);
        assert_eq!(alloc.allocated.get(), false);

        let alloc = Tracking::from(Global);
        {
            let mut vec = Vec::<u8, _>::new_in(&alloc);
            vec.push(0);
        }

        assert_eq!(alloc.allocated.get(), true);
    }

    #[should_panic]
    #[test]
    fn test_must_allocate_panic() {
        must_allocate(Dummy, |_| {});
    }

    #[test]
    fn test_must_allocate_allocated() {
        must_allocate(Global, |alloc| {
            let mut vec = Vec::<u8, _>::new_in(alloc);
            vec.push(0);
        });
    }
}
