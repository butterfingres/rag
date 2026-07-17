use {
    rayon_core::{ThreadPool, ThreadPoolBuilder},
    std::{io::Write as _, sync::Mutex},
};

#[rem::defun(user_ptr)]
pub fn new<'e>(env: &'e rem::Env, pipe_process: rem::Value<'e>) -> Result<ThreadPool, rem::Error> {
    let channel = Mutex::new(env.open_channel(pipe_process)?);
    ThreadPoolBuilder::new()
        .panic_handler(move |panic| {
            let mut channel = channel.lock().unwrap();
            if let Some(msg) = panic.downcast_ref::<String>() {
                let _ = writeln!(channel, "{msg}");
            } else if let Some(msg) = panic.downcast_ref::<&str>() {
                let _ = writeln!(channel, "{msg}");
            } else {
                let _ = writeln!(channel, "panic in pooled thread");
            }
        })
        .build()
        .map_err(rem::Error::from)
}
