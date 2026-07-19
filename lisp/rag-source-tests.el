;;; rag-source-tests.el --- source tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: files, news, data

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))
(require 'ert)

(eval-when-compile (require 'rag-db-tests-lib))
(require 'rag-db)
(require 'rag-source)

(ert-deftest rag-source-tests-handle-new-entry ()
  (rag-db-tests-with db
    (sqlite-execute-batch db
                          "INSERT INTO feed (url)
VALUES ('https://example.com/feed');
INSERT INTO entry (id, pub_date, feed_id)
VALUES ('1', 1782739075, 'https://example.com/feed')")
    (let ((inserted nil)
          (deleted nil))
      (rag-source-handle-new-entry "https://example.com/feed"
                                   db
                                   (lambda (entry)
                                     (setq deleted t)
                                     (should (equal entry
                                                    (make-rag-entry :id "1"
                                                                    :pub-date 1782739075))))
                                   (lambda (entry)
                                     (setq inserted t)
                                     (should (equal entry
                                                    (make-rag-entry :id "1"
                                                                    :pub-date 1782739175
                                                                    :feed-id "https://example.com/feed"))))
                                   (make-rag-entry :id "1"
                                                   :pub-date 1782739175))
      (should (and inserted deleted)))
    (let ((inserted nil))
      (rag-source-handle-new-entry "https://example.com/feed"
                                   db
                                   (lambda (_entry)
                                     (should nil))
                                   (lambda (entry)
                                     (setq inserted t)
                                     (should (equal entry
                                                    (make-rag-entry :id "2"
                                                                    :pub-date 1782739275
                                                                    :feed-id "https://example.com/feed"))))
                                   (make-rag-entry :id "2"
                                                   :pub-date 1782739275))
      (should inserted))))

(ert-deftest rag-source-tests-update ()
  (rag-db-tests-with db
    (cl-letf (((symbol-function #'url-queue-retrieve)
               (lambda (_url cb &optional cbargs _silent _inhibit-cookies)
                 (with-temp-buffer
                   (insert "HTTP 1.1 OK /")
                   (newline)
                   (newline)
                   (insert "<?xml version=\"1.0\"?>
<feed xmlns=\"http://www.w3.org/2005/Atom\" xmlns:foo=\"http://example.com/foo\">
  <title>test feed</title>
  <updated>2003-12-13T18:30:02Z</updated>
  <link rel=\"self\" href=\"https://example.com/atom\"/>
  <link href=\"https://example.com\" rel=\"alternate\"/>
  <entry>
    <title>first entry</title>
    <id>1</id>
    <description>entry number 1</description>
    <foo:content>faux contents of entry number 1</foo:content>
    <content>contents of entry number 1</content>
    <updated>2004-12-13T18:30:02Z</updated>
    <link rel=\"alternate\" href=\"https://example.com/entry_1\"/>
    <link rel=\"enclosure\" href=\"https://example.com/entry_1.mp3\"/>
  </entry>
</feed>")
                   (apply cb '() cbargs)))))
      (let* ((completed nil)

             (progress-buffer (rag-progress-buffer-get))
             (rag-source-update-functions (list (lambda (_to-delete _to-insert)
                                                  (setq completed t)))))
        (unwind-protect
            (with-current-buffer progress-buffer
              (rag-source-update "https://example.com/atom")

              (while (not completed)
                (sit-for 0.1))

              (goto-char (point-min))
              (should (string= (buffer-substring-no-properties (point-min) (point-max))
                               "fetching https://example.com/atom... ok\n"))

              (should (equal (car (sqlite-select db "SELECT * FROM FEED
WHERE url == 'https://example.com/atom'"))
                             ;; description
                             '("https://example.com/atom"
                               ;; title
                               "test feed"
                               ;; link
                               "https://example.com"
                               ;; skip days
                               nil
                               ;; skip hours
                               nil
                               ;; last-update
                               1071340202
                               ;; ttl
                               nil
                               ;; frequency
                               nil)))

              (let ((entry (car (sqlite-select db "SELECT * FROM entry
WHERE id == '1'"))))
                (should (equal entry
                               ;; id
                               '("1"
                                 ;; title
                                 "first entry"
                                 ;; link
                                 "https://example.com/entry_1"
                                 ;; description
                                 "contents of entry number 1"
                                 ;; pub date
                                 1102962602
                                 ;; hidden
                                 0
                                 ;; feed-id
                                 "https://example.com/atom"))))

              (let ((enclosures (mapcar #'car
                                        (sqlite-select db "SELECT link FROM enclosure
WHERE entry_id == '1'"))))
                (should (equal enclosures
                               '("https://example.com/entry_1.mp3")))))
          (kill-buffer progress-buffer))))))

(ert-deftest rag-source-tests-update-cached ()
  (rag-db-tests-with db
    (sqlite-execute db "INSERT INTO feed(url, ttl, last_update) VALUES ('https://example.com/atom', 'P60M', 0)")
    (cl-letf (((symbol-function #'url-queue-retrieve)
               (lambda (_url _cb &optional _cbargs _silent _inhibit-cookies)
                 (should nil)))
              ((symbol-function #'float-time)
               (lambda (&optional _specified-time)
                 0)))
      (let ((progress-buffer (rag-progress-buffer-get)))
        (unwind-protect
            (with-current-buffer progress-buffer
              (rag-source-update "https://example.com/atom")
              (should (string= (buffer-substring-no-properties (point-min) (point-max))
                               "fetching https://example.com/atom... cached\n")))
          (kill-buffer progress-buffer))))))

(provide 'rag-source-tests)

;;; rag-source-tests.el ends here
