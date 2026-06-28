;;; rag-entry-tests.el --- rag-entry tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(require 'ert)

(require 'rag-entry)

(ert-deftest rag-entry-tests-insert-header-empty ()
  (with-temp-buffer
    (rag-entry-insert-header "foo" nil 'foo)
    (should (string= (buffer-string) ""))))

(ert-deftest rag-entry-tests-insert-header ()
  (let ((l (with-temp-buffer
             (rag-entry-insert-header "foo" "bar" 'foo)
             (buffer-string)))
        (r (with-temp-buffer
             (insert "foo: bar")
             (newline)
             (add-text-properties 4 8 '(face foo))
             (buffer-string))))
    (should (string= l r))))

(defun rag-entry-tests-test-renderer (cont)
  "Test renderer in CONT.

CONT will be called with an entry and a callback to be called in the
buffer."
  (funcall cont
           (make-rag-entry :title "entry"
                           :link "https://example.com"
                           :pub-date 1782688282
                           :description "foo"
                           :enclosures '("https://example.com/entry_1.mp3"))
           (lambda ()
             (should (string= (buffer-substring-no-properties (point-min)
                                                              (point-max))
                              "Title: entry
Link: https://example.com
Date: 2026-06-28
Enclosure: https://example.com/entry_1.mp3

foo
")))))

(ert-deftest rag-entry-tests-render ()
  (rag-entry-tests-test-renderer
   (lambda (entry cont)
     (with-temp-buffer
       (rag-entry-render entry)
       (funcall cont)))))

(provide 'rag-entry-tests)

;;; rag-entry-tests.el ends here
