;;; rag-entry.el --- entry viewer -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: news

;;; Commentary:

;;; Code:

(require 'rag-lib)

(defgroup rag-entry '()
  "Entry viewer."
  :group 'rag-entry)

(defcustom rag-entry-buffer-name "*rag-entry*"
  "The name of the entry buffer viewer."
  :type 'string
  :group 'rag-entry)

(defvar rag-entry-mode-entry nil
  "The entry that is currently being viewed.")

(define-derived-mode rag-entry-mode special-mode "RAG Entry"
  :group 'rag-entry
  (setq-local rag-entry-mode-entry rag-entry-mode-entry)
  (save-excursion
    (when-let* ((description (rag-entry-description rag-entry-mode-entry)))
      (let ((inhibit-read-only t)
            (start (point)))
        (insert description)
        (shr-render-region start (point))))))

(defun rag-entry-buffer-get (entry)
  (let ((buffer (get-buffer-create rag-entry-buffer-name)))
    (with-current-buffer buffer
      (unless (derived-mode-p '(rag-entry-mode))
        (let ((rag-entry-mode-entry entry))
          (rag-entry-mode))))
    buffer))

(defun rag-entry-visit (entry)
  "Visit ENTRY in the entry buffer."
  (pop-to-buffer (rag-entry-buffer-get entry)))

(provide 'rag-entry)

;;; rag-entry.el ends here
