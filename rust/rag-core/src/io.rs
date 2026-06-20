use {
    crate::sym,
    emacs::Value,
    std::{
        cmp,
        error::Error,
        fmt::{self, Display, Formatter},
        io::{self, Read},
    },
};

#[derive(Debug)]
struct EmacsError(emacs::Error);
impl Display for EmacsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
impl Error for EmacsError {}

pub struct BufferReader<'e> {
    marker: Value<'e>,
}
impl<'e> BufferReader<'e> {
    pub fn try_new(env: &'e emacs::Env) -> Result<Self, emacs::Error> {
        Ok(Self {
            marker: sym::fun::POINT_MIN_MARKER.call(env, [])?,
        })
    }
}
impl<'e> Read for BufferReader<'e> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        (|| {
            let max = sym::fun::POINT_MAX.call(self.marker.env, [])?;
            if buf.is_empty() || {
                sym::fun::GEQ
                    .call(self.marker.env, (self.marker, max))?
                    .is_not_nil()
            } {
                Ok::<usize, emacs::Error>(0)
            } else {
                let start = sym::fun::MARKER_POSITION
                    .call(self.marker.env, (self.marker,))?
                    .into_rust::<usize>()?;
                let end = max.into_rust::<usize>()?;
                let request = end - start;

                if buf.len() == 1 {
                    let request = cmp::min(request, 1);
                    let substring = sym::fun::BUFFER_SUBSTRING
                        .call(self.marker.env, (start, start + request))?;

                    let mut temp_buf = [0; 2];
                    let read = substring.copy_string_contents(&mut temp_buf)?.len();
                    for i in 0..read {
                        buf[i] = temp_buf[i];
                    }
                    let new_pos = sym::fun::PLUS.call(self.marker.env, (self.marker, read))?;
                    sym::fun::SET_MARKER.call(self.marker.env, (self.marker, new_pos))?;

                    Ok(read)
                } else {
                    // the capacity that can be used by [Value::copy_string_contents].
                    let request = cmp::min(request, buf.len() - 1);
                    let substring = sym::fun::BUFFER_SUBSTRING
                        .call(self.marker.env, (start, start + request))?;
                    let read = substring.copy_string_contents(buf)?.len();
                    let new_pos = sym::fun::PLUS.call(self.marker.env, (self.marker, read))?;
                    sym::fun::SET_MARKER.call(self.marker.env, (self.marker, new_pos))?;
                    Ok(read)
                }
            }
        })()
        // .inspect(|e| eprintln!("read {e}"))
        // .inspect_err(|e| {
        //     eprintln!("error: {e}");
        // })
        .map_err(EmacsError)
        .map_err(io::Error::other)
    }
}

#[cfg(debug_assertions)]
#[emacs::defun(name = "-buffer-string")]
/// Return the the buffer contents as a string.
///
/// This function is only available when compling with debug
/// assertions as this is only intended to be used in tests.
fn buffer_string<'e>(env: &'e emacs::Env) -> Result<String, emacs::Error> {
    let mut buf = String::new();
    let mut reader = std::io::BufReader::new(BufferReader::try_new(env)?);
    reader.read_to_string(&mut buf)?;

    Ok(buf)
}
