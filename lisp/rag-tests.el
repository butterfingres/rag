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

(defmacro rag-tests-with-buffer (buffer &rest body)
  (declare (indent 1))
  `(let ((rag-oldest-entry nil)
         (,buffer (rag-buffer-get)))
     (unwind-protect
         (progn
           ,@body)
       (kill-buffer buffer))))

(defun rag-tests-ui-test (feed entry output)
  (rag-db-tests-with db
   (sqlite-execute db "INSERT INTO feed (url, title) VALUES (?, ?)" feed)
   (sqlite-execute db "INSERT INTO entry (id, title, pub_date, feed_id) VALUES (?, ?, ?, ?)" entry)
   (rag-tests-with-buffer buffer
     (with-current-buffer buffer
       (should (string= (buffer-substring-no-properties (point-min)
                                                        (point-max))
                        output))))))

(ert-deftest rag-tests-happy ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" "example feed")
                       `("1" "entry 1" ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d example feed entry 1\n" date))))

(ert-deftest rag-tests-empty-feed ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" "entry 1" ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty feed title> entry 1\n" date))))

(ert-deftest rag-tests-empty-entry ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" "example feed")
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d example feed <empty entry title>\n" date))))

(ert-deftest rag-tests-empty ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty feed title> <empty entry title>\n" date))))

(ert-deftest rag-tests-entry-at-point ()
  (rag-db-tests-with db
   (sqlite-execute-batch db "INSERT INTO
  feed (url, title)
VALUES
  ('https://example.com/feed', 'example feed');
INSERT INTO
  entry (id, pub_date, feed_id)
VALUES
  ('1', 1782675986, 'https://example.com/feed'),
  ('2', 1782675986, 'https://example.com/feed')")
   (rag-tests-with-buffer buffer
     (with-current-buffer buffer
       (goto-char (point-min))
       (should (equal (rag-entry-at-point)
                      (make-rag-entry :id "1"
                                      :pub-date 1782675986
                                      :feed-id "https://example.com/feed")))
       (goto-line 2)
       (should (equal (rag-entry-at-point)
                      (make-rag-entry :id "2"
                                      :pub-date 1782675986
                                      :feed-id "https://example.com/feed")))))))

(provide 'rag-tests)

;;; rag-tests.el ends here
