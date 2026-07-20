use std::{
    borrow::Cow,
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
};

fn get_binary<'a>(var: &str, default: &'a str) -> Cow<'a, Path> {
    env::var_os(var)
        .map(PathBuf::from)
        .map(Cow::Owned)
        .unwrap_or(Cow::Borrowed(Path::new(default)))
}

fn def_macro(var: &str, val: &Path) -> OsString {
    let mut buf = OsString::from(var);
    buf.push("=");
    buf.push(val);
    buf
}

#[test]
fn lisp_tests() -> Result<(), io::Error> {
    let emacs = get_binary("EMACS", "emacs");
    let make = get_binary("MAKE", "make");

    let librag_core_so = test_cdylib::build_current_project();

    let mut rag_core_so = librag_core_so.clone();
    let extension = librag_core_so.extension().map(OsStr::to_os_string);
    rag_core_so.set_file_name("rag-core");
    if let Some(extension) = extension {
        rag_core_so.set_extension(extension);
    }

    let target_dir = rag_core_so
        .parent()
        .expect("target directory should have a parent");

    let process::Output {
        status: child_status,
        stdout: child_stdout,
        stderr: child_stderr,
    } = Command::new(&*make)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg(def_macro("TARGET_DIR", &target_dir))
        .arg(def_macro("LIBRAG_CORE_SO", &librag_core_so))
        .arg(def_macro("RAG_CORE_SO", &rag_core_so))
        .arg(def_macro("EMACS", &emacs))
        .output()?;

    if !child_status.success() {
        let mut stdout = io::stdout();
        stdout.write_all(&child_stdout)?;
        stdout.flush()?;

        let mut stderr = io::stderr();
        stderr.write_all(&child_stderr)?;
        stderr.flush()?;

        panic!("make exited with non-zero exit code");
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
                    .arg(&target_dir)
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
