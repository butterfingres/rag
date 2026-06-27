;;; rag-fetcher-url-retrieve.el --- url-retreive backend for fetchers -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(require 'rag-fetcher)

(cl-defstruct rag-fetcher-url-retrieve
  "`url-retrieve' backend for fetching.")

(cl-defmethod rag-fetcher-fetch-url (_generic-config
                                     (_config rag-fetcher-url-retrieve)
                                     url
                                     callback)
  (letrec ((buffer (url-retrieve url
                                 (lambda (status)
                                   (unwind-protect
                                       (with-current-buffer buffer
                                         (funcall callback status))
                                     (kill-buffer buffer)))
                                 '()
                                 t)))
    nil))

(provide 'rag-fetcher-url-retrieve)

;;; rag-fetcher-url-retrieve.el ends here
