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
(require 'rag-entry)
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
        (let* ((title (or (when-let* ((title (car entry)))
                            (propertize title
                                        'face 'rag-unread-entry-title))
                          (propertize rag-empty-entry-title
                                      'face 'rag-null)))
               (pub-date (cadr entry))

               (date (format-time-string "%Y-%m-%d" pub-date))

               (feed-id (caddr entry))
               (feed-title (if-let* ((title (caar (sqlite-select db "SELECT title FROM feed
WHERE url == ?1"
                                                                (list feed-id)))))
                               (propertize title
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

(defun rag-entry-at-point ()
  "Get the entry at the current point."
  (when (or (eobp)
            (and (bobp)
                 (eobp)))
    (error "Cannot get entry from empty buffer"))
  (let ((db (rag-db-get))
        (offset (1- (line-number-at-pos))))
    (cl-destructuring-bind (id title link description pub-date feed-id)
        (car (if rag-oldest-entry
                 (sqlite-select db
                                "SELECT * FROM ENTRY
WHERE pub_date > ?
ORDER BY pub_date DESC
LIMIT 1 OFFSET ?"
                                (list (- (round (float-time)) rag-oldest-entry)
                                      offset))
               (sqlite-select db
                              "SELECT * FROM ENTRY
ORDER BY pub_date DESC
LIMIT 1 OFFSET ?"
                              (list offset))))
      (let ((enclosures (mapcar #'car
                                (sqlite-select db
                                               "SELECT link FROM enclosure
WHERE entry_id == ?"
                                               (list id)))))
        (make-rag-entry :title title
                        :link link
                        :description description
                        :id id
                        :pub-date pub-date
                        :enclosures enclosures
                        :feed-id feed-id)))))

(defun rag-visit-entry-at-point ()
  (interactive)
  (let ((entry (rag-entry-at-point)))
    (rag-entry-visit entry)))

(keymap-set rag-mode-map "<return>" #'rag-visit-entry-at-point)

(provide 'rag)

;;; rag.el ends here
