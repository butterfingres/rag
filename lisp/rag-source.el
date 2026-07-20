;;; rag-source.el --- feed sources -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: data, news

;;; Commentary:

;;; Code:

(eval-when-compile
  (require 'cl-macs)
  (require 'sqlite)
  (require 'url-vars))

(eval-and-compile (require 'rag-pool))
(require 'rag-core)
(require 'rag-db)
(require 'rag-progress)
(require 'rag-thread-pool)

(cl-defstruct rag-source
  "Feed source."
  url)

(defgroup rag-source '()
  "Feed sources."
  :group 'rag)

(defcustom rag-source-inhibit-cache nil
  "If non-nil this will not respect caches."
  :group 'rag
  :type 'boolean)

(defcustom rag-source-completion-hook '(rag-core-alloc-clear)
  "A hook that gets run when all feeds are parsed."
  :group 'rag-source
  :type 'hook)

(defcustom rag-source-update-functions '()
  "A hook that gets ran after parsing a feed.

Functions are called with a list of entries to delete and a list of
entries to add to update the ui. The list of deleting entries will at
least have `rag-entry-id' and `rag-entry-pub-date' while to added
entries will have those and at least `rag-entry-title' and
`rag-entry-feed-id'."
  :group 'rag-source
  :type 'hook)

(defcustom rag-source-entry-functions '()
  "A list of functions that gets called with parsed entries."
  :group 'rag-source
  :type 'hook)

(defcustom rag-source-feeds '()
  "A list of feeds to download."
  :type '(repeat string)
  :group 'rag-source)

(defun rag-source-handle-new-entry (url db delete-entry insert-entry entry)
  (let* ((id (or (rag-entry-id entry)
                 (rag-entry-link entry)
                 (format "urn:sha1:%s"
                         (sha1 (or (rag-entry-description entry)
                                   (float-time))))))
         (old-pub-date (car-safe (car (sqlite-select db "SELECT pub_date FROM entry
WHERE id == ?"
                                                     (list id))))))
    (sqlite-execute db
                    "INSERT OR REPLACE INTO entry(id, title, link, description, pub_date, feed_id)
VALUES (?, ?, ?, ?, ?, ?)"
                    (list id
                          (rag-entry-title entry)
                          (rag-entry-link entry)
                          (rag-entry-description entry)
                          (or (rag-entry-pub-date entry)
                              (round (float-time)))
                          url))

    (sqlite-execute db
                    "DELETE FROM enclosure
WHERE entry_id == ?"
                    (list id))
    (cl-loop for enclosure across (rag-entry-enclosures entry)
             do (sqlite-execute db
                                "INSERT INTO enclosure(entry_id, link)
VALUES (?, ?)"
                                (list id enclosure)))
    (run-hook-with-args 'rag-source-entry-functions
                        entry)

    (when old-pub-date
      (funcall delete-entry (make-rag-entry :id id
                                            :pub-date old-pub-date)))
    (cl-destructuring-bind (title pub-date feed-id) (car (sqlite-select db "SELECT title, pub_date, feed_id FROM entry
WHERE id == ?"
                                                                        (list id)))
      (funcall insert-entry (make-rag-entry :id id
                                            :title title
                                            :pub-date pub-date
                                            :feed-id feed-id)))))

(defun rag-source-update-region (url start end)
  (let* ((db (rag-db-get))
         (thread-pool (rag-thread-pool-get))
         (to-delete '())
         (to-insert '())
         (buffer (generate-new-buffer " *rag-source-parser*"))
         (process (make-pipe-process :name "rag-source-parser"
                                     :buffer buffer
                                     :noquery t
                                     :filter (lambda (process text)
                                               (with-current-buffer (process-buffer process)
                                                 (let ((marker (process-mark process)))
                                                   (goto-char marker)
                                                   (insert text)
                                                   (set-marker marker (point)))

                                                 (goto-char (point-min))
                                                 (ignore-error end-of-file
                                                   (while t
                                                     (let ((sexp (read buffer)))
                                                       (delete-region (point-min) (point))
                                                       (cl-case (car sexp)
                                                         (rag-entry
                                                          (rag-source-handle-new-entry
                                                           url
                                                           db
                                                           (lambda (entry) (push entry to-delete))
                                                           (lambda (entry) (push entry to-insert))
                                                           (apply #'record sexp)))
                                                         (rag-feed
                                                          (let* ((feed (apply #'record sexp)))
                                                            (sqlite-execute db
                                                                            "INSERT OR REPLACE INTO feed (url, title, link, skip_days, skip_hours, ttl, last_update)
VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)"
                                                                            (list url
                                                                                  (rag-feed-title feed)
                                                                                  (rag-feed-link feed)
                                                                                  (rag-feed-skip-days feed)
                                                                                  (rag-feed-skip-hours feed)
                                                                                  (rag-feed-ttl feed)
                                                                                  (or (rag-feed-last-update feed)
                                                                                      (round (float-time))))))

                                                          (delete-process process)
                                                          (kill-buffer buffer)

                                                          (run-hook-with-args 'rag-source-update-functions to-delete to-insert))
                                                         (error
                                                          (error "%s" (cadr sexp))))))))))))
    (rag-core-parse-string-with (buffer-substring-no-properties start
                                                                end)
                                thread-pool
                                process)))

(defun rag-source-update (url)
  "Update source URL."
  (let* ((db (rag-db-get))
         (progress-buffer (rag-progress-buffer-get))
         (marker (with-current-buffer progress-buffer
                   (save-excursion
                     (goto-char (point-max))
                     (let ((inhibit-read-only t))
                       (insert "fetching ")
                       (insert (propertize url 'face 'link))
                       (insert "...")
                       (prog1
                           (point-marker)
                         (newline))))))
         (should-fetch (or rag-source-inhibit-cache
                           (if-let* ((row (car (sqlite-select db
                                                              "SELECT ttl, frequency, last_update, skip_days, skip_hours
FROM feed
WHERE url == ?"
                                                              (list url)))))
                               (cl-destructuring-bind (ttl frequency last-update skip-days skip-hours) row
                                 (rag-core-feed-fetch-p ttl frequency last-update skip-days skip-hours (round (float-time))))
                             t))))
    (if should-fetch
        (url-queue-retrieve
         url
         (lambda (status)
           (unwind-protect
               (condition-case error-value
                   (progn
                     (when (eq (car-safe status) :error)
                       (signal (cadadr status)
                               (cddadr status)))

                     (goto-char (point-min))
                     (re-search-forward (rx line-start line-end))
                     (forward-line)

                     (rag-source-update-region url (point) (point-max))

                     (when (buffer-live-p progress-buffer)
                       (with-current-buffer progress-buffer
                         (save-excursion
                           (goto-char marker)
                           (end-of-line)
                           (let ((inhibit-read-only t))
                             (insert " " (propertize "ok" 'face 'success)))))))
                 (error
                  (when (buffer-live-p progress-buffer)
                    (with-current-buffer progress-buffer
                      (save-excursion
                        (goto-char marker)
                        (end-of-line)
                        (let ((inhibit-read-only t))
                          (insert " " (propertize (format "%S" error-value)
                                                  'face 'error))))))))
             (kill-buffer (current-buffer))))
         '()
         t)
      (with-current-buffer progress-buffer
        (goto-char marker)
        (let ((inhibit-read-only t))
          (insert " " (propertize "cached" 'face 'success)))))))

(defun rag-source-update-all ()
  (interactive)
  (dolist (url rag-source-feeds)
    (rag-source-update url))
  (run-hooks 'rag-source-completion-hook))

(provide 'rag-source)

;;; rag-source.el ends here
