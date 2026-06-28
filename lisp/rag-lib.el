;;; rag-lib.el --- rag type definitions and utilities -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 26 Jun 2026
;; Version: 0.1.0
;; Keywords: data

;;; Commentary:

;;; Code:

(eval-when-compile (require 'cl-macs))

(defconst rag-abi-version 0
  "The major abi version of rag.

This is incremented every time there is a breaking change between elisp
and rust code.")

(cl-defstruct rag-feed
  "Feed structure."
  title
  link
  skip-days
  skip-hours
  ttl
  last-update)

(cl-defstruct rag-entry
  "Entry structure."
  title
  link
  description
  id
  pub-date
  enclosures
  feed-id)

(provide 'rag-lib)

;;; rag-lib.el ends here
