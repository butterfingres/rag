;;; rag-db.el --- database implementation -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(defgroup rag-db '()
  "Rag database."
  :group 'rag)

(defcustom rag-db-path (expand-file-name "rag.db" user-emacs-directory)
  "The path to the database."
  :group 'rag-db
  :type 'file)

(defconst rag-db-migrations ["CREATE TABLE schema(
  version INTEGER PRIMARY KEY
);
CREATE TABLE feed(
  url STRING PRIMARY KEY,
  title STRING,
  link STRING,
  skip_days INTEGER,
  skip_hours INTEGER,
  ttl INTEGER,
  last_update INTEGER
)"]
  "A list of sql migrations.

Running every sql snippet in this vector should create the newest
schema.")

(defmacro rag-db-with-transaction (db &rest body)
  "Execute BODY in a transaction in DB.

This will `sqlite-commit' the changes on success and `sqlite-rollback'
on error."
  (declare (indent 1))
  `(let ((db ,db))
     (sqlite-transaction db)
     (condition-case error-value
         (progn
           ,@body
           (sqlite-commit db))
       (error
        (sqlite-rollback db)
        (signal (car error-value) (cdr error-value))))))

(defvar rag-db nil
  "The sqlite database object.")

(defun rag-db-get ()
  "Get the `rag-db'."
  (with-memoization rag-db
    (let* ((new (not (file-exists-p rag-db-path)))
           (db (sqlite-open rag-db-path)))
      (if new
          (rag-db-with-transaction db
            (cl-loop for migration across rag-db-migrations
                     do (sqlite-execute-batch db migration)
                     finally do (sqlite-execute db "INSERT INTO schema(version) VALUES(?1)"
                                                (list (length rag-db-migrations)))))
        (let ((last-version (or (caar (sqlite-select db
                                                     "SELECT MAX(version) FROM schema"))
                                (length rag-db-migrations))))
          (rag-db-with-transaction db
            (cl-loop for migration across (substring rag-db-migrations last-version)
                     with i = 0
                     do (progn
                          (sqlite-execute-batch db migration)
                          (sqlite-execute db
                                          "INSERT INTO schema(version) VALUES(?1)"
                                          (list (+ i last-version)))
                          (setq i (1+ i)))))))
      db)))

(provide 'rag-db)

;;; rag-db.el ends here
