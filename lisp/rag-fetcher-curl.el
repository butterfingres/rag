;;; rag-fetcher-curl.el --- curl fetcher implementation -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 27 Jun 2026
;; Version: 0.1.0
;; Keywords: 

;;; Commentary:

;;; Code:

(require 'rag-fetcher)

(defgroup rag-fetcher-curl '()
  "Curl fetcher backend."
  :group 'rag)

(defcustom rag-fetcher-curl-program (or (executable-find "curl") "curl")
  "The default curl program."
  :group 'rag-fetcher-curl
  :type 'file)

(defcustom rag-fetcher-curl-swiches '("--silent")
  "Parameters to pass before the url."
  :group 'rag-fetcher-curl
  :type '(repeat string))

(cl-defstruct rag-fetcher-curl
  "Curl fetcher configuration."
  program
  switches)

(provide 'rag-fetcher-curl)

;;; rag-fetcher-curl.el ends here
