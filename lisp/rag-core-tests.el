;;; rag-core-tests.el --- tests for rag-core -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 20 Jun 2026
;; Version: 0.1.0
;; Keywords: data

;;; Commentary:

;;; Code:

(require 'ert)
(require 'rag-core)

(defun rag-core-test-parse-feed (input output-feed)
  (let ((feed (rag-core-parse-string input (rag-core-bump-new) (lambda (_entry) nil))))
    (should (equal feed output-feed))))

(ert-deftest rag-core-test-atom-feed ()
  (rag-core-test-parse-feed
   "<?xml version=\"1.0\"?>
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
</feed>"
   (make-rag-feed :title "test feed"
                  :link "https://example.com"
                  :last-update 1071340202)))

(provide 'rag-core-tests)

;;; rag-core-tests.el ends here
