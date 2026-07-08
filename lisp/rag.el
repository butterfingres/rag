;;; rag.el --- rag ui front-end -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 2.0.0
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
(require 'rag-source)

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

(defun rag-entry-insert (entry)
  "Insert ENTRY into the current buffer.

ENTRY must at least contain `rag-entry-title', `rag-entry-pub-date',
`rag-entry-id', and `rag-entry-feed-id'."
  (let* ((db (rag-db-get))
         (title (or (when-let* ((title (rag-entry-title entry)))
                      (propertize title
                                  'face 'rag-unread-entry-title))
                    (propertize rag-empty-entry-title
                                'face 'rag-null)))
         (pub-date (rag-entry-pub-date entry))

         (date (format-time-string "%Y-%m-%d" pub-date))

         (feed-id (rag-entry-feed-id entry))
         (feed-title (if-let* ((title (caar (sqlite-select db "SELECT title FROM feed
WHERE url == ?1"
                                                           (list feed-id)))))
                         (propertize title
                                     'face 'rag-feed-title)
                       (propertize rag-empty-feed-title
                                   'face 'rag-null)))

         (id (rag-entry-id entry))

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
    (add-text-properties start (point) `(rag-entry-id ,id))))

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
        (cl-destructuring-bind (title pub-date feed-id id) entry
          (rag-entry-insert (make-rag-entry :title title
                                            :pub-date pub-date
                                            :feed-id feed-id
                                            :id id))
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
                          id)))

  (save-excursion
    (let* ((start-column 11)
           (start (progn
                    (move-to-column start-column)
                    (point)))
           (end (progn
                  (move-to-column (+ start-column rag-title-width))
                  (point)))
           (inhibit-read-only t))
      (put-text-property start end
                         'face
                         (if hidden
                             'rag-read-entry-title
                           'rag-unread-entry-title)))))

(defun rag-entry-hide-at-point ()
  (interactive)
  (rag-entry-set-hidden-at-point t))

(defun rag-entry-unhide-at-point ()
  (interactive)
  (rag-entry-set-hidden-at-point nil))

(defun rag-visit-entry-at-point ()
  (interactive)
  (let ((entry (rag-entry-at-point)))
    (rag-entry-set-hidden-at-point t)
    (rag-entry-visit entry)))

(keymap-set rag-mode-map "<return>" #'rag-visit-entry-at-point)
(keymap-set rag-mode-map "G" #'rag-source-update-all)
(keymap-set rag-mode-map "r" #'rag-entry-hide-at-point)
(keymap-set rag-mode-map "u" #'rag-entry-unhide-at-point)

(defun rag-binary-search-buffer-desc (value-at-point
                                      value)
  "Binary search for VALUE in a reverse sorted buffer.

VALUE-AT-POINT is a function that returns the value at point.  If the
value is found this function returns (found . LINE) otherwise it
returns (not-found . LINE) where LINE is either the line number to
insert VALUE into or the line number of VALUE."
  (catch 'return
    (let ((low (line-number-at-pos (point-min)))
          (high (1- (line-number-at-pos (point-max)))))
      (while (<= low high)
        (let* ((mid (+ low (/ (- high low) 2)))
               (point-value (progn
                              (goto-char (point-min))
                              (forward-line (1- mid))
                              (funcall value-at-point))))
          (cond
           ((= point-value value)
            (throw 'return `(found . ,mid)))
           ((> point-value value)
            (setq low (1+ mid)))
           (t
            (setq high (1- mid))))))
      `(not-found . ,low))))

(defun rag-entry-id-at-point ()
  (get-text-property (point) 'rag-entry-id))

(defun rag-entry-pub-date-at-point ()
  (caar (sqlite-select (rag-db-get)
                       "SELECT pub_date FROM entry WHERE id == ?"
                       (list (rag-entry-id-at-point)))))

(defun rag-update-function (to-delete to-insert)
  (when-let* ((buffer (get-buffer rag-buffer-name)))
    (with-current-buffer buffer
      (let ((inhibit-read-only t))
        (save-excursion
          (dolist (entry to-delete)
            (pcase (rag-binary-search-buffer-desc #'rag-entry-pub-date-at-point
                                                  (rag-entry-pub-date entry))
              (`(found . ,line)
               (let ((line-point (progn
                                   (goto-char (point-min))
                                   (forward-line (1- line))
                                   (point))))
                 (while (and (not (eobp))
                             (progn
                               (forward-line)
                               (eql (rag-entry-pub-date-at-point) (rag-entry-pub-date entry))))
                   (when (equal (rag-entry-id-at-point)
                                (rag-entry-id entry))
                     (delete-line)))

                 (goto-char line-point)
                 (when (equal (rag-entry-id-at-point)
                              (rag-entry-id entry))
                   (delete-line))

                 (goto-char line-point)
                 (while (and (not (bobp))
                             (progn
                               (forward-line -1)
                               (eql (rag-entry-pub-date-at-point) (rag-entry-pub-date entry))))
                   (when (equal (rag-entry-id-at-point)
                                (rag-entry-id entry))
                     (delete-line)))))))

          (dolist (entry to-insert)
            (let ((line (cdr (rag-binary-search-buffer-desc #'rag-entry-pub-date-at-point
                                                            (rag-entry-pub-date entry)))))
              (goto-char (point-min))
              (forward-line (1- line))
              (open-line 1)
              (rag-entry-insert entry))))))))

(add-to-list 'rag-source-update-functions #'rag-update-function)

(provide 'rag)

;;; rag.el ends here
