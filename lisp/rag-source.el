;;; rag-source.el --- feed sources -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: data, news

;;; Commentary:

;;; Code:

(eval-when-compile
  (require 'cl-macs)
  (require 'sqlite))

(eval-when-compile (require 'rag-pool))
(require 'rag-db)
(require 'rag-progress)

(cl-defstruct rag-source
  "Feed source."
  url
  tags)

;; taken from `org-id-uuid'
(defun rag-source--uuid ()
  "Return string with random (version 4) UUID."
  (let ((rnd (md5 (format "%s%s%s%s%s%s%s"
			              (random)
			              (time-convert nil 'list)
			              (user-uid)
			              (emacs-pid)
			              (user-full-name)
			              user-mail-address
			              (recent-keys)))))
    (format "%s-%s-4%s-%s%s-%s"
	        (substring rnd 0 8)
	        (substring rnd 8 12)
	        (substring rnd 13 16)
	        (format "%x"
		            (logior
		             #b10000000
		             (logand
		              #b10111111
		              (string-to-number
		               (substring rnd 16 18) 16))))
	        (substring rnd 18 20)
	        (substring rnd 20 32))))

(defun rag-source-update-region (source start end)
  (rag-pool-with alloc
    (let ((db (rag-db-get)))
      (with-sqlite-transaction db
        (let* ((string (buffer-substring start end))
               (feed (rag-core-parse-string
                      string
                      alloc
                      (lambda (entry)
                        (let ((id (or (rag-entry-id entry)
                                      (rag-source--uuid))))
                          (sqlite-execute db
                                          "INSERT OR REPLACE INTO entry(id, title, link, description, pub_date)
VALUES (?, ?, ?, ?, ?)"
                                          (list id
                                                (rag-entry-title entry)
                                                (rag-entry-link entry)
                                                (rag-entry-description entry)
                                                (rag-entry-pub-date entry)))
                          (cl-loop for enclosure across (rag-entry-enclosures entry)
                                   do (sqlite-execute db
                                                      "INSERT INTO enclosure(entry_id, link)
VALUES (?, ?)"
                                                      (list id enclosure))))))))
          (sqlite-execute db
                          "INSERT OR REPLACE INTO feed(url, title, link, skip_days, skip_hours, ttl, last_update) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)"
                          (list (rag-source-url source)
                                (rag-feed-title feed)
                                (rag-feed-link feed)
                                (rag-feed-skip-days feed)
                                (rag-feed-skip-hours feed)
                                (rag-feed-ttl feed)
                                (rag-feed-last-update feed))))))))

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
