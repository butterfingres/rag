;;; rag-pool-tests.el --- pool tests -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: 

;;; Commentary:

;;; Code:

(require 'ert)

(require 'rag-pool)

(ert-deftest rag-pool-tests-reuse ()
  (let ((alloc (rag-core-bump-new)))
    (push alloc rag-pool-allocators)
    (rag-pool-with pooled-alloc
      (should (eq alloc pooled-alloc)))
    (should (eq (car rag-pool-allocators) alloc))))

(provide 'rag-pool-tests)

;;; rag-pool-tests.el ends here
