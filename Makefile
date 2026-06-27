.POSIX:

ELCS = lisp/rag-lib.elc lisp/rag-core-tests.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L target/debug -L lisp

CARGO = cargo

PREFIX = /usr/local
SITELISP = ${PREFIX}/share/emacs/site-lisp

# find -name '*.toml' -o -name '*.rs' | sed 's/\.\///g' | sort | xargs echo
RUSTFILES = Cargo.toml src/alloc.rs src/borrow.rs src/bump.rs src/fmt.rs src/lib.rs src/num.rs src/sym.rs src/tz.rs src/xml/atom/mod.rs src/xml/mod.rs src/xml/parser.rs src/xml/rdf/mod.rs src/xml/rss/mod.rs

include Makefile.in

all: ${ELCS}
check: all
	${EMACS} ${EMACSFLAGS} -l rag-core-tests -l ert -f ert-run-tests-batch-and-exit
	${CARGO} ${CARGOFLAGS} test ${CARGOTESTFLAGS}
clean:
	-rm ${ELCS}
	-${CARGO} ${CARGOFLAGS} clean ${CARGOCLEANFLAGS}

target/debug/${LIB}: ${RUSTFILES}
	${CARGO} ${CARGOFLAGS} build ${CARGOBUILDFLAGS}
target/debug/rag-core.${SO}: target/debug/librag_core.${SO}
	cp $< $@

target/release/${LIB}: ${RUSTFILES}
	${CARGO} ${CARGOFLAGS} build --release ${CARGOBUILDFLAGS}
target/release/rag-core.${SO}: target/release/librag_core.${SO}
	cp $< $@

lisp/rag-core-tests.elc: lisp/rag-lib.elc target/debug/rag-core.so

install: target/release/rag-core.${SO} ${ELCS}
	install -m 755 -d "${SITELISP}"
	install -m 644 target/release/rag-core.${SO} lisp/*.el lisp/*.elc "${SITELISP}"

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
