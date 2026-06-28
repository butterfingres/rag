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

(defcustom rag-title-align 50
  "How many characters to align the title to."
  :group 'rag
  :type 'natnum)

;; taken from elfeed-search.el
(defface rag-date
  '((((class color) (background light)) (:foreground "#aaa"))
    (((class color) (background dark))  (:foreground "#77a")))
  "Face used in search mode for dates."
  :group 'rag)

(defface rag-unread-title
  '((t :weight bold))
  "Face used in search mode for unread entry titles."
  :group 'rag)

(defface rag-feed
  '((((class color) (background light)) (:foreground "#aa0"))
    (((class color) (background dark))  (:foreground "#ff0")))
  "Face used in search mode for feed titles."
  :group 'rag)

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil
  (let ((db (rag-db-get)))
    (goto-char (point-min))
    (dolist (entry (sqlite-select db "SELECT title, pub_date, feed_id FROM entry
ORDER BY pub_date DESC"))
      (let* ((title (car entry))
             (pub-date (cadr entry))

             (date (format-time-string "%Y-%m-%d" pub-date))

             (feed-id (caddr entry))
             (feed-title (caar (sqlite-select db "SELECT title FROM feed
WHERE url == ?1"
                                              (list feed-id))))

             (inhibit-read-only t))
        (insert (propertize date
                            'face 'rag-date)
                " "
                (propertize title
                            'face 'rag-unread-title)
                (propertize " "
                            'display `(space :align-to ,rag-title-align))
                (propertize feed-title
                            'face 'rag-feed))
        (newline)))))

(add-hook 'rag-mode-hook #'toggle-truncate-lines)

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
