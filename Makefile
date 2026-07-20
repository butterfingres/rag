.POSIX:

ELCS = lisp/rag-core-tests.elc lisp/rag-db-tests-lib.elc				\
	lisp/rag-db-tests.elc lisp/rag-db.elc lisp/rag-entry-tests.elc		\
	lisp/rag-entry.elc lisp/rag-faces.elc lisp/rag-lib.elc				\
	lisp/rag-pool-tests.elc lisp/rag-pool.elc lisp/rag-progress.elc		\
	lisp/rag-source-tests.elc lisp/rag-source.elc lisp/rag-tests.elc	\
	lisp/rag-thread-pool.elc lisp/rag.elc

TARGET_DIR = target/debug
LIBRAG_CORE_SO = ${TARGET_DIR}/librag_core.so
RAG_CORE_SO = ${TARGET_DIR}/rag-core.so

EMACS = emacs
EMACSFLAGS = -Q -batch -L ${TARGET_DIR} -L lisp

all: ${ELCS} ${RAG_CORE_SO}
clean:
	-rm ${ELCS}
	cargo clean

${RAG_CORE_SO}: ${LIBRAG_CORE_SO}
	ln $< $@

lisp/rag.elc: lisp/rag-db.elc lisp/rag-entry.elc lisp/rag-faces.elc lisp/rag-pool.elc lisp/rag-source.elc ${RAG_CORE_SO}
lisp/rag-core-tests.elc: lisp/rag-pool.elc
lisp/rag-entry.elc: lisp/rag-faces.elc lisp/rag-lib.elc
lisp/rag-entry-tests.elc: lisp/rag-entry.elc
lisp/rag-tests.elc: lisp/rag.elc lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-source.elc: lisp/rag-db.elc lisp/rag-pool.elc lisp/rag-progress.elc ${RAG_CORE_SO}
lisp/rag-source-tests.elc: lisp/rag-source.elc lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-pool.elc: ${RAG_CORE_SO}
lisp/rag-pool-tests.elc: lisp/rag-pool.elc
lisp/rag-db-tests.elc: lisp/rag-db.elc lisp/rag-db-tests-lib.elc
lisp/rag-db-tests-lib.elc: lisp/rag-db.elc
lisp/rag-thread-pool.elc: ${RAG_CORE_SO}
lisp/rag-core-tests.elc: ${RAG_CORE_SO}

.el.elc:
	${EMACS} ${EMACSFLAGS} -l bytecomp -f batch-byte-compile $<
.SUFFIXES: .el .elc

.PHONY: all clean
