use std::fmt::{self, Display, Formatter, Write as _};

pub fn debug_bytes<T>(bytes: &T, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
where
    T: AsRef<[u8]> + ?Sized,
{
    f.write_str("b\"")?;
    for chunk in bytes.as_ref().utf8_chunks() {
        for ch in chunk.valid().chars() {
            Display::fmt(&ch.escape_debug(), f)?;
        }
        for byte in chunk.invalid() {
            Display::fmt(&byte.escape_ascii(), f)?;
        }
    }
    f.write_char('\"')?;

    Ok(())
}
pub fn debug_optional_bytes<T>(bytes: &Option<T>, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
where
    T: AsRef<[u8]>,
{
    if let Some(bytes) = bytes.as_ref() {
        f.write_str("Some(")?;
        debug_bytes(bytes, f)?;
        f.write_str(")")?;
    } else {
        f.write_str("None")?;
    }

    Ok(())
}
pub fn debug_iter_bytes<T, U>(iter: &T, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
where
    T: AsRef<[U]> + ?Sized,
    U: AsRef<[u8]>,
{
    f.write_str("[")?;
    let mut first = true;
    for s in iter.as_ref() {
        if first {
            first = false;
        } else {
            f.write_str(", ")?;
        }
        debug_bytes(s, f)?;
    }
    f.write_str("]")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use {super::*, arrayvec::ArrayString};

    fn test_formatter_fn<const N: usize, F, G, T>(
        format: F,
        input: G,
        output: &str,
    ) -> Result<(), fmt::Error>
    where
        F: Fn(T, &mut Formatter<'_>) -> Result<(), fmt::Error>,
        G: Fn() -> T,
    {
        let mut buf = ArrayString::<N>::new();
        write!(buf, "{}", fmt::from_fn(move |f| format(input(), f)))?;
        assert_eq!(buf.as_str(), output);
        Ok(())
    }

    #[test]
    fn test_debug_bytes_empty() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"b"""#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(debug_bytes, || b"", OUTPUT)
    }

    #[test]
    fn test_debug_bytes_unescaped() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"b"hello""#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(debug_bytes, || b"hello", OUTPUT)
    }

    #[test]
    fn test_debug_bytes_escape_ascii() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"b"\x9d""#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(debug_bytes, || b"\x9d", OUTPUT)
    }

    #[test]
    fn test_debug_bytes_escape_debug() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"b"\n""#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(debug_bytes, || b"\n", OUTPUT)
    }

    #[test]
    fn test_debug_optional_bytes_some() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"Some(b"hello")"#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(
            debug_optional_bytes,
            || &Some(b"hello"),
            OUTPUT,
        )
    }

    #[test]
    fn test_debug_optional_bytes_none() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"None"#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(
            debug_optional_bytes,
            || -> &Option<&[u8]> { &None },
            OUTPUT,
        )
    }

    #[test]
    fn test_debug_iter_bytes_empty() -> Result<(), fmt::Error> {
        const OUTPUT: &str = "[]";
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(
            debug_iter_bytes,
            || -> &[&[u8]] { &[] },
            OUTPUT,
        )
    }

    #[test]
    fn test_debug_iter_bytes_single() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"[b"hello"]"#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(
            debug_iter_bytes,
            || -> &[&[u8]] { &[b"hello"] },
            OUTPUT,
        )
    }

    #[test]
    fn test_debug_iter_bytes_double() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"[b"hello", b"world"]"#;
        test_formatter_fn::<{ OUTPUT.len() }, _, _, _>(
            debug_iter_bytes,
            || -> &[&[u8]] { &[b"hello", b"world"] },
            OUTPUT,
        )
    }
}
