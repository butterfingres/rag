;;; rag-db.el --- database implementation -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(eval-when-compile
  (require 'cl-macs)
  (require 'sqlite))

(defgroup rag-db '()
  "Rag database."
  :group 'rag)

(defcustom rag-db-path (expand-file-name "rag.db" user-emacs-directory)
  "The path to the database."
  :group 'rag-db
  :type '(choice file
                 (const nil)))

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
);
CREATE TABLE entry(
  id TEXT PRIMARY KEY,
  title TEXT,
  link TEXT,
  description TEXT,
  pub_date INTEGER NOT NULL,
  feed_id STRING,
  FOREIGN KEY (feed_id) REFERENCES feed(url)
);
CREATE TABLE enclosure(
  entry_id TEXT NOT NULL,
  link TEXT,
  FOREIGN KEY (entry_id) REFERENCES entry(id)
)"]
  "A list of sql migrations.

Running every sql snippet in this vector should create the newest
schema.")

(defvar rag-db nil
  "The sqlite database object.")

(defun rag-db-get ()
  "Get the `rag-db'."
  (with-memoization rag-db
    (let* ((new (if (stringp rag-db-path)
                    (not (file-exists-p rag-db-path))
                  t))
           (db (sqlite-open rag-db-path)))
      (if new
          (with-sqlite-transaction db
            (cl-loop for migration across rag-db-migrations
                     do (sqlite-execute-batch db migration)
                     finally do (sqlite-execute db "INSERT INTO schema(version) VALUES(?1)"
                                                (list (length rag-db-migrations)))))
        (let ((last-version (or (caar (sqlite-select db
                                                     "SELECT MAX(version) FROM schema"))
                                (length rag-db-migrations))))
          (with-sqlite-transaction db
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
