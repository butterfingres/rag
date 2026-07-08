.POSIX:

ELCS = lisp/rag-core-tests.elc lisp/rag-db-tests-lib.elc				\
	lisp/rag-db-tests.elc lisp/rag-db.elc lisp/rag-entry-tests.elc		\
	lisp/rag-entry.elc lisp/rag-faces.elc lisp/rag-lib.elc				\
	lisp/rag-pool-tests.elc lisp/rag-pool.elc lisp/rag-progress.elc		\
	lisp/rag-source-tests.elc lisp/rag-source.elc lisp/rag-tests.elc	\
	lisp/rag.elc

EMACS = emacs
EMACSFLAGS = -Q -batch -L target/debug -L lisp -eval '(setq byte-compile-error-on-warn t)'

CARGO = cargo
CARGOCLIPPYFLAGS = --all-targets --all-features -- -D warnings

PREFIX = /usr/local
SITELISP = ${PREFIX}/share/emacs/site-lisp
LIBDIR = ${PREFIX}/share/emacs/site-lisp

RUSTFILES = Cargo.lock Cargo.toml src/alloc.rs src/borrow.rs		\
	src/bump.rs src/feed.rs src/fmt.rs src/lib.rs src/num.rs		\
	src/sym.rs src/tz.rs src/xml/fmt/atom/mod.rs src/xml/fmt/mod.rs	\
	src/xml/fmt/rdf/mod.rs src/xml/fmt/rss/mod.rs src/xml/mod.rs	\
	src/xml/ns/content/mod.rs src/xml/ns/dc/mod.rs					\
	src/xml/ns/media/mod.rs src/xml/ns/mod.rs src/xml/ns/sy/mod.rs	\
	src/xml/parser.rs

include Makefile.in

all: ${ELCS}
check: all
	[ "$$(find Cargo.* src -name 'Cargo.*' -o -name '*.rs' | sort | xargs echo)" = "$$(echo ${RUSTFILES})" ] || (echo 'Go update $${RUSTFILES}.' 1>&2; exit 1)
	[ "$$(find lisp -name '*.el' | sed 's/\.el/\.elc/g' | sort | xargs echo)" = "$$(echo ${ELCS})" ] || (echo 'Go update $${ELCS}.' 1>&2; exit 1)
	${EMACS} ${EMACSFLAGS} -l rag-core-tests   -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-db-tests     -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-entry-tests  -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-pool-tests   -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-source-tests -l ert -f ert-run-tests-batch-and-exit
	${EMACS} ${EMACSFLAGS} -l rag-tests        -l ert -f ert-run-tests-batch-and-exit
	${CARGO} ${CARGOFLAGS} fmt    ${CARGOFMTFLAGS}    --check
	${CARGO} ${CARGOFLAGS} check  ${CARGOCHECKFLAGS}
	${CARGO} ${CARGOFLAGS} clippy ${CARGOCLIPPYFLAGS}
	${CARGO} ${CARGOFLAGS} test   ${CARGOTESTFLAGS}
clean:
	-rm ${ELCS}
	-${CARGO} ${CARGOFLAGS} clean ${CARGOCLEANFLAGS}

target/debug/${LIB}: ${RUSTFILES}
	${CARGO} ${CARGOFLAGS} build ${CARGOBUILDFLAGS}
target/debug/rag-core.${SO}: target/debug/${LIB}
	cp $< $@

target/release/${LIB}: ${RUSTFILES}
	${CARGO} ${CARGOFLAGS} build --release ${CARGOBUILDFLAGS}
target/release/rag-core.${SO}: target/release/${LIB}
	cp $< $@

lisp/rag.elc: lisp/rag-db.elc lisp/rag-entry.elc lisp/rag-faces.elc lisp/rag-lib.elc lisp/rag-pool.elc lisp/rag-source.elc target/debug/rag-core.${SO}
lisp/rag-entry.elc: lisp/rag-faces.elc lisp/rag-lib.elc
lisp/rag-entry-tests.elc: lisp/rag-entry.elc
lisp/rag-tests.elc: lisp/rag.elc lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-source.elc: lisp/rag-db.elc lisp/rag-lib.elc lisp/rag-pool.elc lisp/rag-progress.elc target/debug/rag-core.${SO}
lisp/rag-source-tests.elc: lisp/rag-source.elc lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-pool.elc: lisp/rag-lib.elc target/debug/rag-core.${SO}
lisp/rag-pool-tests.elc: lisp/rag-pool.elc
lisp/rag-db-tests.elc: lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-db-tests-lib.elc: lisp/rag-db.elc
lisp/rag-core-tests.elc: lisp/rag-lib.elc target/debug/rag-core.${SO}

install: target/release/rag-core.${SO} ${ELCS}
	install -m 755 -d "${SITELISP}"
	install -m 644 lisp/*.el lisp/*.elc "${SITELISP}"
	install -m 755 -d "${LIBDIR}"
	install -m 644 target/release/rag-core.${SO} "${LIBDIR}"

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all check clean
