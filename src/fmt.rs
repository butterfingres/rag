use std::fmt::{self, Display, Formatter, Write as _};

pub fn debug_bytes<T>(bytes: &T, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
where
    T: AsRef<[u8]> + ?Sized,
{
    f.write_str("b\"")?;
    for chunk in bytes.as_ref().utf8_chunks() {
        f.write_str(chunk.valid())?;
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

    fn test_debug_iter_bytes<const BUF: usize>(
        params: &[&[u8]],
        output: &str,
    ) -> Result<(), fmt::Error> {
        let mut buf = ArrayString::<BUF>::new();
        write!(buf, "{}", fmt::from_fn(|f| { debug_iter_bytes(params, f) }))?;
        assert_eq!(buf.as_str(), output);

        Ok(())
    }

    #[test]
    fn test_debug_iter_bytes_empty() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"[]"#;
        test_debug_iter_bytes::<{ OUTPUT.len() }>(&[], OUTPUT)
    }

    #[test]
    fn test_debug_iter_bytes_single() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"[b"hello"]"#;
        test_debug_iter_bytes::<{ OUTPUT.len() }>(&[b"hello"], OUTPUT)
    }

    #[test]
    fn test_debug_iter_bytes_double() -> Result<(), fmt::Error> {
        const OUTPUT: &str = r#"[b"hello", b"world"]"#;
        test_debug_iter_bytes::<{ OUTPUT.len() }>(&[b"hello", b"world"], OUTPUT)
    }
}
