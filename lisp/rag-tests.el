;;; rag-tests.el --- rag tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 1.0.0
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
                       (format-time-string "%Y-%m-%d entry 1                                            example feed\n" date))))

(ert-deftest rag-tests-empty-feed ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" "entry 1" ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d entry 1                                            <empty feed title>\n" date))))

(ert-deftest rag-tests-empty-entry ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" "example feed")
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty entry title>                                example feed\n" date))))

(ert-deftest rag-tests-empty ()
  (let ((date 1782571353))
    (rag-tests-ui-test '("https://example.com/feed" nil)
                       `("1" nil ,date "https://example.com/feed")
                       (format-time-string "%Y-%m-%d <empty entry title>                                <empty feed title>\n" date))))

(ert-deftest rag-tests-entry-set-hidden-at-point ()
  (rag-db-tests-with db
    (sqlite-execute-batch db "INSERT INTO feed (url, title)
VALUES ('https://example.com/feed', 'example feed');
INSERT INTO entry (id, pub_date, feed_id)
VALUES ('1', 1782675986, 'https://example.com/feed')")
    (rag-tests-with-buffer buffer
      (with-current-buffer buffer
        (goto-char (point-min))
        (rag-entry-set-hidden-at-point t)
        (should (= (caar (sqlite-select db "SELECT hidden FROM entry"))
                   1))
        (rag-entry-set-hidden-at-point nil)
        (should (= (caar (sqlite-select db "SELECT hidden FROM entry"))
                   0))))))

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
       (forward-line)
       (should (equal (rag-entry-at-point)
                      (make-rag-entry :id "2"
                                      :pub-date 1782675986
                                      :feed-id "https://example.com/feed")))))))

(ert-deftest rag-tests-update-function-delete ()
  (rag-db-tests-with db
    (sqlite-execute-batch db "INSERT INTO
  feed (url, title)
VALUES
  ('https://example.com/feed', 'example feed');
INSERT INTO
  entry (id, pub_date, feed_id)
VALUES
  ('1', 1782675986, 'https://example.com/feed')")
    (rag-tests-with-buffer buffer
      (with-current-buffer buffer
        (rag-update-function (list (make-rag-entry :id "1"
                                                   :pub-date 1782675986))
                             '())
        (should (string= (buffer-string) ""))
        (rag-update-function '()
                             (list (make-rag-entry :id "1"
                                                   :title "hello world"
                                                   :feed-id "https://example.com/feed"
                                                   :pub-date 1782675986)))
        (should (string= (buffer-substring-no-properties (point-min) (point-max))
                         "2026-06-28 hello world                                        example feed
"))))))

(ert-deftest rag-tests-update-function-insert ()
  (rag-db-tests-with db
    (sqlite-execute-batch db "INSERT INTO
  feed (url, title)
VALUES
  ('https://example.com/feed', 'example feed');
INSERT INTO
  entry (id, title, pub_date, feed_id)
VALUES
  ('1', '1', 1782675988, 'https://example.com/feed'),
  ('2', '2', 1782675987, 'https://example.com/feed'),
  ('4', '4', 1782675985, 'https://example.com/feed')")
    (rag-tests-with-buffer buffer
      (with-current-buffer buffer
        (rag-update-function '()
                             (list (make-rag-entry :id "3"
                                                   :title "3"
                                                   :feed-id "https://example.com/feed"
                                                   :pub-date 1782675986)))
        (should (string= (buffer-substring-no-properties (point-min) (point-max))
                         "2026-06-28 1                                                  example feed
2026-06-28 2                                                  example feed
2026-06-28 3                                                  example feed
2026-06-28 4                                                  example feed
"))))))

(provide 'rag-tests)

;;; rag-tests.el ends here
