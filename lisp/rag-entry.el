;;; rag-entry.el --- entry viewer -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 1.0.0
;; Keywords: news

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(require 'rag-faces)
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

(defun rag-entry-insert-header (name value face)
  (when value
    (insert name ": " (propertize value 'face face))
    (newline)))

(defun rag-entry-render (entry)
  (save-excursion
    (let ((inhibit-read-only t))
      (erase-buffer)
      (rag-entry-insert-header "Title" (rag-entry-title entry) 'rag-feed-title)
      (rag-entry-insert-header "Link" (rag-entry-link entry) 'link)
      (rag-entry-insert-header "Date"
                               (when-let* ((pub-date (rag-entry-pub-date entry)))
                                 (format-time-string "%Y-%m-%d" pub-date))
                               'rag-date)
      (cl-loop for enclosure in (rag-entry-enclosures entry)
               do (progn
                    (insert "Enclosure: " (propertize enclosure 'face 'link))
                    (newline)))
      (when-let* ((description (rag-entry-description entry)))
        (newline)
        (let ((start (point)))
          (insert description)
          (shr-render-region start (point)))))))

(define-derived-mode rag-entry-mode special-mode "RAG Entry"
  :group 'rag-entry)

(defun rag-entry-buffer-get (entry)
  (let ((buffer (get-buffer-create rag-entry-buffer-name)))
    (with-current-buffer buffer
      (unless (derived-mode-p '(rag-entry-mode))
        (rag-entry-mode))
      (rag-entry-render entry))
    buffer))

(defun rag-entry-visit (entry)
  "Visit ENTRY in the entry buffer."
  (pop-to-buffer (rag-entry-buffer-get entry)))

(provide 'rag-entry)

;;; rag-entry.el ends here
