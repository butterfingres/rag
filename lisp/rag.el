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

(defcustom rag-oldest-entry (* 60 60 24
                               30
                               6)
  "How many seconds to no longer show entries in the feed.

Set to nil if to never exclude entries based on age."
  :group 'rag
  :type '(choice natnum
                 (const nil)))

(defcustom rag-title-align 50
  "How many characters to align the title to."
  :group 'rag
  :type 'natnum)

(defcustom rag-empty-entry-title "<empty entry title>"
  "The title text of empty entries."
  :type 'string
  :group 'rag)

(defcustom rag-empty-feed-title "<empty feed title>"
  "The title text of empty"
  :type 'string
  :group 'rag)

;; taken from elfeed-search.el
(defface rag-date
  '((((class color) (background light)) (:foreground "#aaa"))
    (((class color) (background dark))  (:foreground "#77a")))
  "Face used in search mode for dates."
  :group 'rag)

(defface rag-unread-entry-title
  '((t :weight bold))
  "Face used in search mode for unread entry titles."
  :group 'rag)

(defface rag-null
  '((t :inherit shadow
       :slant italic))
  "Face used for null text."
  :group 'rag)

(defface rag-feed-title
  '((((class color) (background light)) (:foreground "#aa0"))
    (((class color) (background dark))  (:foreground "#ff0")))
  "Face used in search mode for feed titles."
  :group 'rag)

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil
  (let ((db (rag-db-get)))
    (save-excursion
      (goto-char (point-min))
      (dolist (entry (if rag-oldest-entry
                         (sqlite-select db "SELECT title, pub_date, feed_id FROM entry
WHERE pub_date > ?1
ORDER BY pub_date DESC"
                                        (list (- (round (float-time)) rag-oldest-entry)))
                       (sqlite-select db "SELECT title, pub_date, feed_id FROM entry
ORDER BY pub_date DESC")))
        (let* ((title (or (propertize (car entry)
                                      'face 'rag-unread-entry-title)
                          (propertize rag-empty-entry-title
                                      'face 'rag-null)))
               (pub-date (cadr entry))

               (date (format-time-string "%Y-%m-%d" pub-date))

               (feed-id (caddr entry))
               (feed-title (if-let* ((title (car (sqlite-select db "SELECT title FROM feed
WHERE url == ?1"
                                                                (list feed-id)))))
                               (propertize (car title)
                                           'face 'rag-feed-title)
                             (propertize rag-empty-feed-title
                                         'face 'rag-null)))

               (inhibit-read-only t))
          (insert (propertize date
                              'face 'rag-date)
                  " "
                  title
                  (propertize " "
                              'display `(space :align-to ,rag-title-align))
                  feed-title)
          (newline))))))

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
