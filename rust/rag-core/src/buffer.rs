use {
    crate::sym,
    emacs::Value,
    std::io::{self, Read},
};

pub struct BufferReader<'e> {
    marker: Value<'e>,
}
impl<'e> BufferReader<'e> {
    pub fn try_new(env: &'e emacs::Env) -> Result<Self, emacs::Error> {
        Ok(Self {
            marker: sym::fun::POINT_MIN_MARKER.call(env, [])?,
        })
    }

    fn read_n_in(
        &mut self,
        request: emacs::Value<'e>,
        buf: &mut [u8],
        start: emacs::Value<'e>,
    ) -> Result<usize, emacs::Error> {
        let read = sym::fun::BUFFER_SUBSTRING
            .call(
                self.marker.env,
                (
                    start,
                    sym::fun::PLUS.call(self.marker.env, (start, request))?,
                ),
            )?
            .copy_string_contents(buf)?
            .len();
        let new_pos = sym::fun::PLUS.call(self.marker.env, (self.marker, read))?;
        sym::fun::SET_MARKER.call(self.marker.env, (self.marker, new_pos))?;

        Ok(read)
    }
}
impl<'e> Read for BufferReader<'e> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        (|| {
            let end = sym::fun::POINT_MAX.call(self.marker.env, [])?;
            if buf.is_empty() || {
                sym::fun::GEQ
                    .call(self.marker.env, (self.marker, end))?
                    .is_not_nil()
            } {
                Ok::<usize, emacs::Error>(0)
            } else {
                let start = sym::fun::MARKER_POSITION.call(self.marker.env, (self.marker,))?;
                let request = sym::fun::MINUS.call(self.marker.env, (end, start))?;

                if buf.len() == 1 {
                    let mut proxy_buf = [0; 2];
                    let read = self.read_n_in(
                        sym::fun::MIN.call(self.marker.env, (request, 1))?,
                        &mut proxy_buf,
                        start,
                    )?;
                    buf[0] = proxy_buf[0];

                    Ok(read)
                } else {
                    self.read_n_in(
                        sym::fun::MIN.call(self.marker.env, (request, buf.len() - 1))?,
                        buf,
                        start,
                    )
                }
            }
        })()
        .map_err(io::Error::other)
    }
}

#[cfg(debug_assertions)]
#[emacs::defun(name = "-string")]
/// Return the the buffer contents as a string.
///
/// This function is only available when compling with debug
/// assertions as this is only intended to be used in tests.  If you
/// need to get a buffer's string efficiently, use `buffer-substring'.
fn buffer_string<'e>(env: &'e emacs::Env) -> Result<String, emacs::Error> {
    let mut buf = String::new();
    let mut reader = std::io::BufReader::new(BufferReader::try_new(env)?);
    reader.read_to_string(&mut buf)?;

    Ok(buf)
}
