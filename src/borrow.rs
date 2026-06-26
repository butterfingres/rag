use {
    allocator_api2::{
        alloc::{Allocator, Global},
        boxed::Box,
        collections::TryReserveError,
        vec::Vec,
    },
    std::{borrow::Borrow, fmt, ops::Deref},
};

pub trait ToOwnedIn<A>
where
    A: Allocator,
{
    type Owned;

    fn try_to_owned_in(&self, _: A) -> Result<Self::Owned, TryReserveError>;
}

impl<A, T> ToOwnedIn<A> for [T]
where
    A: Allocator,
    T: Clone,
{
    type Owned = Vec<T, A>;

    fn try_to_owned_in(&self, alloc: A) -> Result<Self::Owned, TryReserveError> {
        let mut vec = Vec::new_in(alloc);
        vec.try_reserve(self.len())?;
        vec.extend(self.iter().cloned());

        Ok(vec)
    }
}

pub enum Cow<'a, T, A = Global>
where
    T: ToOwnedIn<A> + ?Sized,
    A: Allocator,
{
    Borrowed(&'a T),
    Owned(T::Owned),
}
impl<'a, T, A> Cow<'a, T, A>
where
    T: ToOwnedIn<A> + ?Sized,
    A: Allocator,
{
    pub fn try_from_in(cow: std::borrow::Cow<'a, T>, alloc: A) -> Result<Self, TryReserveError>
    where
        T: ToOwned,
        <T as ToOwned>::Owned: Borrow<T>,
    {
        match cow {
            std::borrow::Cow::Borrowed(val) => Ok(Self::Borrowed(val)),
            std::borrow::Cow::Owned(val) => Ok(Self::Owned(val.borrow().try_to_owned_in(alloc)?)),
        }
    }
    pub fn try_to_mut_in(&mut self, alloc: A) -> Result<&mut T::Owned, TryReserveError> {
        match self {
            Self::Borrowed(val) => {
                *self = Cow::Owned(val.try_to_owned_in(alloc)?);
                match self {
                    Self::Borrowed(_) => unreachable!(),
                    Self::Owned(val) => Ok(val),
                }
            }
            Self::Owned(val) => Ok(val),
        }
    }
}
impl<'a, T, A> Clone for Cow<'a, T, A>
where
    T: ToOwnedIn<A> + ?Sized,
    T::Owned: Clone,
    A: Allocator,
{
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(val) => Cow::Borrowed(*val),
            Self::Owned(val) => Cow::Owned(val.clone()),
        }
    }
}
impl<'a, T, A> AsRef<T> for Cow<'a, T, A>
where
    T: ToOwnedIn<A> + PartialEq + ?Sized + 'a,
    T::Owned: AsRef<T>,
    A: Allocator,
{
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(val) => val,
            Self::Owned(val) => val.as_ref(),
        }
    }
}
impl<'a, T, A> Deref for Cow<'a, T, A>
where
    T: ToOwnedIn<A> + PartialEq + ?Sized + 'a,
    T::Owned: AsRef<T>,
    A: Allocator,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}
impl<T, A> fmt::Debug for Cow<'_, T, A>
where
    T: fmt::Debug + ToOwnedIn<A, Owned: fmt::Debug> + ?Sized,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Cow::Borrowed(ref b) => fmt::Debug::fmt(b, f),
            Cow::Owned(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}
impl<'a, T, A> Default for Cow<'a, [T], A>
where
    [T]: ToOwnedIn<A>,
    A: Allocator,
{
    fn default() -> Self {
        Self::Borrowed(&[])
    }
}
impl<'a, T, A1, A2> PartialEq<Cow<'a, T, A2>> for Cow<'a, T, A1>
where
    T: ToOwnedIn<A1> + ToOwnedIn<A2> + PartialEq + ?Sized + 'a,
    <T as ToOwnedIn<A1>>::Owned: AsRef<T>,
    <T as ToOwnedIn<A2>>::Owned: AsRef<T>,
    A1: Allocator,
    A2: Allocator,
{
    fn eq(&self, r: &Cow<'a, T, A2>) -> bool {
        self.as_ref() == r.as_ref()
    }
}

pub enum MaybeOwned<'a, T, A = Global>
where
    T: ?Sized,
    A: Allocator,
{
    Borrow(&'a T),
    Owned(Box<T, A>),
}
