//! Elisp values.

pub struct Value<'e> {
    data: emacs_value,
    _marker: PhantomData<&'a ()>,
}
