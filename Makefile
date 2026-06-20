.POSIX:

ELCS = lisp/rag-core-tests.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L target/debug -L lisp

CARGO = cargo

all: ${ELCS}
check: all
	${EMACS} ${EMACSFLAGS} -l rag-core-tests -l ert -f ert-run-tests-batch-and-exit
	${CARGO} ${CARGOFLAGS} test
clean:
	-rm ${ELCS}
	-${CARGO} ${CARGOFLAGS} clean

target/debug/librag_core.so: Cargo.toml src/buffer.rs src/lib.rs src/sym.rs
	${CARGO} ${CARGOFLAGS} build
target/debug/rag-core.so: target/debug/librag_core.so
	ln -sf $$(realpath $<) $@

lisp/rag-core-tests.elc: target/debug/rag-core.so

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
