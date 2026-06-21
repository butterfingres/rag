use allocator_api2::{alloc::Allocator, collections::TryReserveError, vec::Vec};

pub trait ToOwnedIn<A>
where
    A: Allocator,
{
    type Owned;

    fn to_owned_in(&self, _: A) -> Result<Self::Owned, TryReserveError>;
}

impl<A, T> ToOwnedIn<A> for [T]
where
    A: Allocator,
    T: Clone,
{
    type Owned = Vec<T, A>;

    fn to_owned_in(&self, alloc: A) -> Result<Self::Owned, TryReserveError> {
        let mut vec = Vec::new_in(alloc);
        vec.try_reserve(self.len())?;
        vec.extend(self.iter().cloned());

        Ok(vec)
    }
}

#[derive(Debug, PartialEq)]
pub enum Cow<'a, T, A>
where
    T: ToOwnedIn<A> + ?Sized,
    A: Allocator,
{
    Borrowed(&'a T),
    Owned(T::Owned),
}
