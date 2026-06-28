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
</rdf:RDF>")
                  (apply cb '() cbargs)))))
     (rag-source-update (make-rag-source :url "https://example.com/rdf"))
     (with-current-buffer (rag-progress-buffer-get)
       (goto-char (point-min))
       (should (string= (buffer-substring-no-properties (point-min) (point-max))
                        "fetching https://example.com/rdf... ok\n")))

     (should (equal (car (sqlite-select db "SELECT title, link FROM FEED
WHERE url == 'https://example.com/rdf'"))
                    '("test feed"
                      "https://example.com")))

     (let* ((entry (car (sqlite-select db "SELECT id, title, link, description FROM entry
LIMIT 1")))
            (id (car entry))
            (body (cdr entry)))
       (should (equal body
                      '("entry 1"
                        "https://example.com/entry_1"
                        "entry 1 description")))))))

(provide 'rag-source-tests)

;;; rag-source-tests.el ends here
