;;; rag-fetcher.el --- fetching module -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(cl-defstruct rag-fetcher-config
  "Fetcher generic fetcher configuration."
  timeout)

(cl-defgeneric rag-fetcher-fetch-url (fetcher-config backend-config url callback)
  "Fetch URL.

FETCHER-CONFIG is the `rag-fetcher-config' configuration for generic
options.  BACKEND-CONFIG is the backend configuration options that also
signals which backend to use.  Call CALLBACK in the buffer when
completed.")

(provide 'rag-fetcher)

;;; rag-fetcher.el ends here
