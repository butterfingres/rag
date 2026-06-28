;;; rag.el --- rag ui front-end -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 0.1.0
;; Keywords: news

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))
(require 'url-queue)

(require 'rag-core)
(require 'rag-db)
(require 'rag-pool)

(defgroup rag '()
  "Rust news AGgragator."
  :group 'news)

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil
  (let ((db (rag-db-get)))
    (dolist (entry (sqlite-select db "SELECT title, pub_date FROM entry
ORDER BY pub_date DESC"))
      (let ((title (car entry))
            (pub-date (cadr entry))
            (inhibit-read-only t))
        (insert (format-time-string "%Y-%m-%d" pub-date)
                " "
                title)
        (newline)))))

(defcustom rag-buffer-name "*rag*"
  "The buffer name to use."
  :group 'rag
  :type 'string)

(defun rag-buffer-get ()
  (or (get-buffer rag-buffer-name)
      (let ((buffer (get-buffer-create rag-buffer-name)))
        (with-current-buffer buffer
          (rag-mode))
        buffer)))

(defun rag-buffer-goto ()
  (interactive)
  (pop-to-buffer (rag-buffer-get)))

(provide 'rag)

;;; rag.el ends here
