.POSIX:

ELCS = lisp/rag-core-tests.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L target/debug -L lisp

CARGO = cargo

include Makefile.in

all: ${ELCS}
check: all
	${EMACS} ${EMACSFLAGS} -l rag-core-tests -l ert -f ert-run-tests-batch-and-exit
	${CARGO} ${CARGOFLAGS} test ${CARGOTESTFLAGS}
clean:
	-rm ${ELCS}
	-${CARGO} ${CARGOFLAGS} clean ${CARGOCLEANFLAGS}

target/debug/${LIB}: Cargo.toml src/alloc.rs src/borrow.rs src/fmt.rs src/lib.rs src/num.rs src/sym.rs src/tz.rs src/xml/atom/mod.rs src/xml/mod.rs src/xml/rdf/mod.rs src/xml/rss/mod.rs
	${CARGO} ${CARGOFLAGS} build ${CARGOBUILDFLAGS}
target/debug/rag-core.${SO}: target/debug/librag_core.${SO}
	ln -sf $$(realpath $<) $@

lisp/rag-core-tests.elc: target/debug/rag-core.so

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
