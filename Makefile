.POSIX:

ELCS = lisp/rag-core-tests.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L rust/rag-core/target/debug -L lisp

CARGO = cargo
CARGOFLAGS = --manifest-path=rust/rag-core/Cargo.toml

all: ${ELCS}
check: all
	${EMACS} ${EMACSFLAGS} -l rag-core-tests -l ert -f ert-run-tests-batch-and-exit
	${CARGO} test ${CARGOFLAGS}
clean:
	-rm ${ELCS}
	-${CARGO} clean ${CARGOFLAGS}

rust/rag-core/target/debug/librag_core.so: rust/rag-core/Cargo.toml rust/rag-core/src/io.rs rust/rag-core/src/lib.rs rust/rag-core/src/sym.rs
	${CARGO} build ${CARGOFLAGS}
rust/rag-core/target/debug/rag-core.so: rust/rag-core/target/debug/librag_core.so
	ln -sf $$(realpath $<) $@

lisp/rag-core-tests.elc: rust/rag-core/target/debug/rag-core.so

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
