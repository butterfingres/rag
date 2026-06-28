;;; rag-source.el --- feed sources -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: data, news

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(eval-when-compile (require 'rag-pool))
(require 'rag-db)
(require 'rag-progress)

(cl-defstruct rag-source
  "Feed source."
  url
  tags)

(defun rag-source-update-region (source start end)
  (rag-pool-with alloc
    (let* ((string (buffer-substring start end))
           (feed (rag-core-parse-string string alloc (lambda (_entry) nil)))
           (db (rag-db-get)))
      (sqlite-execute db
                      "INSERT OR REPLACE INTO feed(url, title, link, skip_days, skip_hours, ttl, last_update) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)"
                      (list (rag-source-url source)
                            (rag-feed-title feed)
                            (rag-feed-link feed)
                            (rag-feed-skip-days feed)
                            (rag-feed-skip-hours feed)
                            (rag-feed-ttl feed)
                            (rag-feed-last-update feed))))))

(defun rag-source-update (source)
  "Update source SOURCE."
  (let* ((progress-buffer (rag-progress-buffer-get))
         (marker (with-current-buffer progress-buffer
                   (save-excursion
                     (goto-char (point-max))
                     (prog1
                         (point-marker)
                       (let ((inhibit-read-only t))
                         (insert "fetching ")
                         (insert (propertize (rag-source-url source) 'face 'link))
                         (insert "...")
                         (newline)))))))
    (url-queue-retrieve
     (rag-source-url source)
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

                 (rag-source-update-region source (point) (point-max))

                 (with-current-buffer (rag-progress-buffer-get)
                   (save-excursion
                     (goto-char marker)
                     (end-of-line)
                     (let ((inhibit-read-only t))
                       (insert " " (propertize "ok" 'face 'success))))))
             (error
              (with-current-buffer (rag-progress-buffer-get)
                (save-excursion
                  (goto-char marker)
                  (end-of-line)
                  (let ((inhibit-read-only t))
                    (insert " " (propertize (apply #'format
                                                   (cdr error-value))
                                            'face 'error)))))))
         (kill-buffer (current-buffer))))
     '()
     t)))

(provide 'rag-source)

;;; rag-source.el ends here
