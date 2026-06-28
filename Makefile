.POSIX:

# find -not -name '.*' -name '*.el' | sed 's/\.\///g' | sed 's/\.el/\.elc/g' | sort | xargs echo
ELCS = lisp/rag-core-tests.elc lisp/rag-db.elc lisp/rag-db-tests.elc lisp/rag-lib.elc lisp/rag-pool.elc lisp/rag.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L target/debug -L lisp

CARGO = cargo

PREFIX = /usr/local
SITELISP = ${PREFIX}/share/emacs/site-lisp
LIBDIR = ${PREFIX}/share/emacs/site-lisp

# find -name '*.toml' -o -name '*.rs' | sed 's/\.\///g' | sort | xargs echo
RUSTFILES = Cargo.toml src/alloc.rs src/borrow.rs src/bump.rs src/fmt.rs src/lib.rs src/num.rs src/sym.rs src/tz.rs src/xml/atom/mod.rs src/xml/mod.rs src/xml/parser.rs src/xml/rdf/mod.rs src/xml/rss/mod.rs

include Makefile.in

all: ${ELCS}
check: all
	${EMACS} ${EMACSFLAGS} -l rag-core-tests -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-db-tests -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-pool-tests -l ert -f ert-run-tests-batch-and-exit
	${CARGO} ${CARGOFLAGS} fmt --check ${CARGOFMTFLAGS}
	${CARGO} ${CARGOFLAGS} check ${CARGOCHECKFLAGS}
	${CARGO} ${CARGOFLAGS} clippy ${CARGOCLIPPYFLAGS}
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

lisp/rag.elc: lisp/rag-db.elc lisp/rag-lib.elc lisp/rag-pool.elc target/debug/rag-core.so
lisp/rag-db-tests.elc: lisp/rag-db.elc
lisp/rag-pool.elc: target/debug/rag-core.so
lisp/rag-pool-tests.elc: lisp/rag-pool.elc
lisp/rag-core-tests.elc: lisp/rag-lib.elc target/debug/rag-core.so

install: target/release/rag-core.${SO} ${ELCS}
	install -m 755 -d "${SITELISP}"
	install -m 644 lisp/*.el lisp/*.elc "${SITELISP}"
	install -m 755 -d "${LIBDIR}"
	install -m 644 target/release/rag-core.${SO} "${LIBDIR}"

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
