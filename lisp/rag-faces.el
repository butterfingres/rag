;;; rag-faces.el --- faces -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: faces

;;; Commentary:

;; Some of the faces are taken from elfeed.

;;; Code:

(defface rag-date
  '((((class color) (background light)) (:foreground "#aaa"))
    (((class color) (background dark))  (:foreground "#77a")))
  "Face used in search mode for dates."
  :group 'rag)

(defface rag-unread-entry-title
  '((t :weight bold))
  "Face used in search mode for unread entry titles."
  :group 'rag)

(defface rag-read-entry-title
  '((((class color) (background light)) (:foreground "#000"))
    (((class color) (background dark))  (:foreground "#fff")))
  "Face used in search mode for read titles."
  :group 'elfeed)

(defface rag-null
  '((t :inherit shadow
       :slant italic))
  "Face used for null text."
  :group 'rag)

(defface rag-feed-title
  '((((class color) (background light)) (:foreground "#aa0"))
    (((class color) (background dark))  (:foreground "#ff0")))
  "Face used in search mode for feed titles."
  :group 'rag)

(provide 'rag-faces)

;;; rag-faces.el ends here
