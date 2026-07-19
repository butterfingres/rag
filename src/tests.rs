use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

#[test]
fn lisp_tests() -> Result<(), io::Error> {
    let emacs = env::var_os("EMACS")
        .map(PathBuf::from)
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(Path::new("emacs")));

    let librag_core_so = test_cdylib::build_current_project();

    let mut rag_core_so = librag_core_so.clone();
    let extension = librag_core_so.extension().map(OsStr::to_os_string);
    rag_core_so.set_file_name("rag-core");
    if let Some(extension) = extension {
        rag_core_so.set_extension(extension);
    }
    if !rag_core_so.exists() {
        fs::hard_link(librag_core_so, &rag_core_so)?;
    }

    let mut children = Vec::new();
    for dirent in fs::read_dir("lisp")? {
        let dirent = dirent?;
        let name = dirent.file_name();
        if name.to_string_lossy().ends_with("-tests.el") {
            children.push(
                Command::new(&*emacs)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .arg("-Q")
                    .arg("-batch")
                    .arg("-l")
                    .arg("ert")
                    .arg("-L")
                    .arg(
                        rag_core_so
                            .parent()
                            .expect("rag-core should have a parent directory"),
                    )
                    .arg("-L")
                    .arg("lisp")
                    .arg("-l")
                    .arg(name)
                    .arg("-f")
                    .arg("ert-run-tests-batch-and-exit")
                    .spawn()?,
            );
        }
    }

    let mut failed = 0;
    for child in children {
        let process::Output {
            status,
            stdout: child_stdout,
            stderr: child_stderr,
        } = child.wait_with_output()?;
        if !status.success() {
            let mut stdout = io::stdout();
            stdout.write_all(&child_stdout)?;
            stdout.flush()?;

            let mut stderr = io::stderr();
            stderr.write_all(&child_stderr)?;
            stderr.flush()?;

            failed += 1;
        }
    }
    if failed != 0 {
        panic!("{failed} child processes failed with non-zero exit status");
    }

    Ok(())
}
