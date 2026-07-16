use {
    crate::sym,
    bump_scope::{FixedBumpString, traits::BumpAllocatorTypedScope},
    rem::{Env, Value},
    std::slice,
};

pub fn from_lisp_in<'a, 'e, A>(
    env: &'e Env,
    val: Value<'e>,
    alloc: A,
) -> Result<FixedBumpString<'a>, rem::Error>
where
    A: BumpAllocatorTypedScope<'a>,
{
    let len = sym::fun::STRING_BYTES
        .try_bind(env)?
        .call(env, (val,))?
        .into_rust::<usize>(env)?;
    let mut string = FixedBumpString::try_with_capacity_in(len + 1, alloc)?;

    let buf = unsafe { string.as_mut_vec() };
    let cap = buf.capacity();
    let slice = unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr(), cap) };
    let len = val.copy_string_contents(env, slice)?.len();
    unsafe {
        buf.set_len(len);
    }

    Ok(string)
}
