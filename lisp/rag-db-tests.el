;;; rag-db-tests.el --- database tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(require 'ert)

(require 'rag-db)

(ert-deftest rag-db-test-schema ()
  (let* ((rag-db-path (make-temp-name (temporary-file-directory)))
         (db (rag-db-get)))
    (unwind-protect
        (progn
          (should (= (caar (sqlite-select db "SELECT MAX(version) FROM SCHEMA"))
                     2))
          (should (= (sqlite-execute db "INSERT INTO feed(url) VALUES('https://example.com/rss')")
                     1)))
      (sqlite-close db)
      (setq rag-db nil)
      (delete-file rag-db-path))))

(provide 'rag-db-tests)

;;; rag-db-tests.el ends here
