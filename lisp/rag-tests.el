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

(ert-deftest rag-tests-ui ()
  (rag-db-tests-with db
    (sqlite-execute-batch
     db
     "INSERT INTO feed (url, title) VALUES ('https://example.com/rss', 'example rss feed');
INSERT INTO entry (id, title, pub_date, feed_id) VALUES ('1', 'entry 1', 1782671353, 'https://example.com/rss')")
    (let ((rag-oldest-entry nil))
      (with-current-buffer (rag-buffer-get)
        (goto-char (point-min))
        (should (string= (buffer-substring-no-properties (point-min) (point-max))
                         (format "%s entry 1 example rss feed\n"
                                 (format-time-string "%Y-%m-%d" 1782671353))))))))

(provide 'rag-tests)

;;; rag-tests.el ends here
