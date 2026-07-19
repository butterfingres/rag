use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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

    for dirent in fs::read_dir("lisp")? {
        let dirent = dirent?;
        let name = dirent.file_name();
        if name.to_string_lossy().ends_with("-tests.el") {
            assert!(
                Command::new(&*emacs)
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
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
                    .status()?
                    .success()
            );
        }
    }

    Ok(())
}
