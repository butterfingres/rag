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

(defun rag-core-tests-test-buffer-string (input)
  (with-temp-buffer
    (insert input)
    (should (string= input (rag-core-buffer--string)))))

(ert-deftest rag-core-tests-buffer-string-empty ()
  (rag-core-tests-test-buffer-string ""))

(ert-deftest rag-core-tests-buffer-string-word ()
  (rag-core-tests-test-buffer-string "hello"))

(ert-deftest rag-core-tests-buffer-string-paragraph ()
  (rag-core-tests-test-buffer-string "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim
veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea
commodo consequat. Duis aute irure dolor in reprehenderit in voluptate
velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint
occaecat cupidatat non proident, sunt in culpa qui officia deserunt
mollit anim id est laborum."))

(provide 'rag-core-tests)

;;; rag-core-tests.el ends here
