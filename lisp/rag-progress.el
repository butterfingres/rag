;;; rag-progress.el --- progress buffer -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 2.0.0
;; Keywords: data

;;; Commentary:

;;; Code:

(define-derived-mode rag-progress-mode special-mode "RAG Progress"
  "Rag progress mode."
  :interactive nil)

(defgroup rag-progress '()
  "Rag progress mode."
  :group 'rag)

(defcustom rag-progress-buffer-name "*rag-progress*"
  "The buffer to use for showing progress."
  :group 'rag-progress
  :type 'string)

(defun rag-progress-buffer-get ()
  "Get the progress buffer."
  (declare (ftype (function () buffer)))
  (or (get-buffer rag-progress-buffer-name)
      (let ((buffer (generate-new-buffer rag-progress-buffer-name)))
        (with-current-buffer buffer
          (rag-progress-mode))
        buffer)))

(defun rag-progress-buffer-goto ()
  "Pop to the progress buffer."
  (interactive)
  (pop-to-buffer (rag-progress-buffer-get)))

(provide 'rag-progress)

;;; rag-progress.el ends here
