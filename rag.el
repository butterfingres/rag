;;; rag.el --- rust news aggragator -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 02 Jun 2026
;; Version: 0.1.0
;; Keywords: news

;;; Commentary:

;;; Code:

(defgroup rag '()
  "Rust news AGgragator."
  :group 'news)

(defcustom rag-db-path (expand-file-name "rag.db" user-emacs-directory)
  "The path to the database."
  :group 'rag
  :type 'file)

(defcustom rag-feeds '()
  "A list of feeds to follow.

An item should either be the url of the feed or a list of the url
followed by the initial tags to apply to the entries of the tag."
  :group 'rag
  :type '(repeat (cons string
                       (repeat symbol))))

(provide 'rag)

;;; rag.el ends here
