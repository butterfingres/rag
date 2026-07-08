;;; rag-db-tests-lib.el --- run database tests utilities -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: data

;;; Commentary:

;;; Code:

(require 'rag-db)

(defmacro rag-db-tests-with (db &rest body)
  (declare (indent 1))
  `(let* ((rag-db-path nil)
          (rag-db nil)
          (,db (rag-db-get)))
     (unwind-protect
         (progn ,@body)
       (sqlite-close ,db))))

(provide 'rag-db-tests-lib)

;;; rag-db-tests-lib.el ends here
