;;; rag-source-tests.el --- source tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: files, news, data

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))
(require 'ert)

(eval-when-compile (require 'rag-db-tests-lib))
(require 'rag-db)
(require 'rag-source)

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
     (rag-source-update (make-rag-source :url "https://example.com/atom"))
     (with-current-buffer (rag-progress-buffer-get)
       (goto-char (point-min))
       (should (string= (buffer-substring-no-properties (point-min) (point-max))
                        "fetching https://example.com/atom... ok\n")))

     (should (equal (car (sqlite-select db "SELECT * FROM FEED
WHERE url == 'https://example.com/atom'"))
                    ;; description
                    '("https://example.com/atom"
                      ;; title
                      "test feed"
                      ;; link
                      "https://example.com"
                      ;; skip days
                      0
                      ;; skip hours
                      0
                      ;; ttl
                      nil
                      ;; last-update
                      1071340202)))

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
                      '("https://example.com/entry_1.mp3")))))))

(provide 'rag-source-tests)

;;; rag-source-tests.el ends here
