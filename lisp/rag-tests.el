;;; rag-tests.el --- rag tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: data

;;; Commentary:

;;; Code:

(require 'ert)

(require 'rag)
(require 'rag-db)
(eval-when-compile (require 'rag-db-tests-lib))

(defun rag-tests-ui-test (feed entry output)
  (rag-db-tests-with db
   (sqlite-execute db "INSERT INTO feed (url, title) VALUES (?, ?)" feed)
   (sqlite-execute db "INSERT INTO entry (id, title, pub_date, feed_id) VALUES (?, ?, ?, ?)" entry)
   (let ((rag-oldest-entry nil)
         (buffer (rag-buffer-get)))
     (unwind-protect
         (with-current-buffer buffer
           (should (string= (buffer-substring-no-properties (point-min)
                                                            (point-max))
                            output)))
       (kill-buffer buffer)))))

(ert-deftest rag-tests-happy ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" "example feed")
                       `("1" "entry 1" ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d entry 1 example feed\n" date))))

(ert-deftest rag-tests-empty-feed ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" "entry 1" ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d entry 1 <empty feed title>\n" date))))

(ert-deftest rag-tests-empty-entry ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" "example feed")
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty entry title> example feed\n" date))))

(ert-deftest rag-tests-empty ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty entry title> <empty feed title>\n" date))))

(provide 'rag-tests)

;;; rag-tests.el ends here
