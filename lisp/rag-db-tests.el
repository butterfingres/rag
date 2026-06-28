;;; rag-db-tests.el --- database tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(require 'ert)

(eval-when-compile (require 'rag-db-tests-lib))
(require 'rag-db)

(ert-deftest rag-db-test-create ()
  (rag-db-tests-with db
    (should (= (caar (sqlite-select db "SELECT MAX(version) FROM SCHEMA"))
               (length rag-db-migrations)))
    (should (= (sqlite-execute db "INSERT INTO feed(url) VALUES('https://example.com/rss')")
               1))))

(ert-deftest rag-db-test-update ()
  (rag-db-tests-with db
    (setq rag-db nil)
    (rag-db-get)
    (should (= (caar (sqlite-select db "SELECT MAX(version) FROM SCHEMA"))
               (length rag-db-migrations)))))

(provide 'rag-db-tests)

;;; rag-db-tests.el ends here
