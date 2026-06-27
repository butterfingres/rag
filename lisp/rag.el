;;; rag.el --- rag ui front-end -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 0.1.0
;; Keywords: news

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))
(require 'url-queue)

(defgroup rag '()
  "Rust news AGgragator."
  :group 'news)

(cl-defstruct rag-source
  "Feed source."
  url
  tags)

(defun rag-source-update (source)
  "Update source SOURCE."
  (letrec ((buffer (url-queue-retrieve
                    (rag-source-url source)
                    (lambda (status)
                      (unwind-protect
                          nil
                        (kill-buffer buffer))))))))

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil)

(provide 'rag)

;;; rag.el ends here
