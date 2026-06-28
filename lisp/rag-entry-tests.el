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

(provide 'rag-entry-tests)

;;; rag-entry-tests.el ends here
