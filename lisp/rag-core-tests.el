;;; rag-core-tests.el --- tests for rag-core -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 20 Jun 2026
;; Version: 2.0.0
;; Keywords: data

;;; Commentary:

;;; Code:

(require 'ert)
(require 'rag-core)
(require 'rag-pool)

(defun rag-core-test-parse-feed (input output-feed output-entries)
  (rag-pool-with alloc
    (let* ((entries '())
           (feed (rag-core-parse-string input
                                        alloc
                                        (lambda (entry)
                                          (push entry entries)))))
      (setq entries (nreverse entries))
      (should (equal feed output-feed))
      (should (equal entries output-entries)))))

;;; Format tests to ensure that all parsers work.

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
                  :last-update 1071340202)
   (list (make-rag-entry :title "first entry"
                         :link "https://example.com/entry_1"
                         :id "1"
                         :description "contents of entry number 1"
                         :pub-date 1102962602
                         :enclosures ["https://example.com/entry_1.mp3"]))))

(ert-deftest rag-core-test-rdf-feed ()
  (rag-core-test-parse-feed
   "<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
         xmlns=\"http://purl.org/rss/1.0/\">
  <channel>
    <title>test feed</title>
    <link>https://example.com</link>
    <description>test feed description</description>
  </channel>
  <item>
    <title>entry 1</title>
    <link>https://example.com/entry_1</link>
    <description>entry 1 description</description>
  </item>
</rdf:RDF>"
   (make-rag-feed :title "test feed"
                  :link "https://example.com")
   (list (make-rag-entry :title "entry 1"
                         :link "https://example.com/entry_1"
                         :description "entry 1 description"))))

(ert-deftest rag-core-test-rss-feed ()
  (rag-core-test-parse-feed
   "<?xml version=\"1.0\"?>
<rss version=\"2.0\">
  <channel>
    <title>example feed</title>
    <link>https://example.com</link>
    <pubDate>Tue, 10 Jun 2003 04:00:00 GMT</pubDate>
    <lastBuildDate>Fri, 21 Jul 2023 09:04 EDT</lastBuildDate>
    <skipHours>
      <hour>1</hour>
      <hour>2</hour>
      <hour>3</hour>
    </skipHours>
    <skipDays>
      <day>Monday</day>
      <day>Tuesday</day>
      <day>Wednesday</day>
      <day>Thursday</day>
      <day>Friday</day>
      <day>Saturday</day>
      <day>Sunday</day>
    </skipDays>
    <ttl>30</ttl>
    <item>
      <title>entry 1</title>
      <link>https://example.com/entry_1</link>
      <description>the first entry</description>
      <guid isPermalink=\"false\">1</guid>
      <pubDate>Fri, 20 Jun 2003 09:00:00 GMT</pubDate>
      <enclosure url=\"https://example.com/entry_1.mp3\"/>
      <enclosure url=\"\"/>
    </item>
    <item>
      <guid>https://example.com/entry_2</guid>
    </item>
  </channel>
</rss>"
   (make-rag-feed :title "example feed"
                  :link "https://example.com"
                  :last-update 1689944640
                  :skip-hours #b00001110
                  :skip-days  #b01111111
                  :ttl "PT30M")
   (list (make-rag-entry :title "entry 1"
                         :link "https://example.com/entry_1"
                         :description "the first entry"
                         :id "1"
                         :pub-date 1056099600
                         :enclosures ["https://example.com/entry_1.mp3" ""])
         (make-rag-entry :id "https://example.com/entry_2"
                         :link "https://example.com/entry_2"))))

(provide 'rag-core-tests)

;;; rag-core-tests.el ends here
