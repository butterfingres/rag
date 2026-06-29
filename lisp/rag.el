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
(require 'rag-faces)
(require 'rag-db)
(require 'rag-pool)

(defgroup rag '()
  "Rust news AGgragator."
  :group 'news)

(defcustom rag-buffer-name "*rag*"
  "The buffer name to use."
  :group 'rag
  :type 'string)

(defcustom rag-oldest-entry (* 60 60 24
                               30
                               6)
  "How many seconds to no longer show entries in the feed.

Set to nil if to never exclude entries based on age."
  :group 'rag
  :type '(choice natnum
                 (const nil)))

(defcustom rag-show-all nil
  "Whether to show hidden entries."
  :group 'rag
  :type 'boolean)

(defcustom rag-title-width 50
  "How many glyphs the title can take before being truncated."
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

(defun rag-render ()
  (let ((db (rag-db-get))
        (inhibit-read-only t))
    (save-excursion
      (erase-buffer)
      (goto-char (point-min))
      (dolist (entry (sqlite-select db "SELECT title, pub_date, feed_id, id FROM entry
WHERE pub_date > ? AND (? OR NOT hidden)
ORDER BY pub_date DESC"
                                    (list (or (and rag-oldest-entry (- (round (float-time)) rag-oldest-entry))
                                              rag-show-all
                                              -1.0e+INF))))
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

               (id (cadddr entry))

               (start (point)))
          (insert (propertize date
                              'face 'rag-date)
                  " "
                  (truncate-string-to-width
                   title
                   rag-title-width
                   0
                   (eval-when-compile (string-to-char " "))
                   "...")
                  " "
                  feed-title)
          (add-text-properties start (point) `(rag-entry-id ,id))
          (newline))))))

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil
  (setq-local revert-buffer-function
              (lambda (&optional _ignore-auto _noconfirm)
                (let ((point (point)))
                  (rag-render)
                  (goto-char point))))
  (rag-render))

(add-hook 'rag-mode-hook #'toggle-truncate-lines)

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
        (id (get-text-property (point) 'rag-entry-id)))
    (cl-destructuring-bind (title link description pub-date feed-id)
        (car (sqlite-select db
                            "SELECT title, link, description, pub_date, feed_id FROM entry
WHERE id == ?"
                            (list id)))
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

(defun rag-entry-set-hidden-at-point (hidden)
  (let* ((db (rag-db-get))
         (id (get-text-property (point) 'rag-entry-id)))
    (sqlite-execute db
                    "UPDATE entry
SET hidden = ?
WHERE id == ?"
                    (list (if hidden
                              1
                            0)
                          id))))

(defun rag-visit-entry-at-point ()
  (interactive)
  (let ((entry (rag-entry-at-point)))
    (rag-entry-visit entry)))

(keymap-set rag-mode-map "<return>" #'rag-visit-entry-at-point)

(provide 'rag)

;;; rag.el ends here
