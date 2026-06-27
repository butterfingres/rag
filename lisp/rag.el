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

;;; Retrieval & progress

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

(cl-defstruct rag-source
  "Feed source."
  url
  tags)

(defun rag-source-parse-buffer ()
  (goto-char (point-min))
  (re-search-forward (rx line-start line-end))
  (forward-line))

(defun rag-source-update-region (start end)
  nil)

(defun rag-source-update (source)
  "Update source SOURCE."
  (let* ((progress-buffer (rag-progress-buffer-get))
         (marker (with-current-buffer (rag-progress-buffer-get)
                   (save-excursion
                     (goto-char (point-max))
                     (prog1
                         (point-marker)
                       (let ((inhibit-read-only t))
                         (insert "fetching ")
                         (insert (propertize (rag-source-url source) 'face 'link))
                         (insert "...")
                         (newline)))))))
    (url-queue-retrieve
     (rag-source-url source)
     (lambda (status)
       (unwind-protect
           (condition-case error-value
               (progn
                 (when (eq (car status) :error)
                   (signal (cadadr status)
                           (cddadr status)))

                 (goto-char (point-min))
                 (re-search-forward (rx line-start line-end))
                 (forward-line)

                 (rag-source-update-region (point) (point-max))

                 (with-current-buffer (rag-progress-buffer-get)
                   (save-excursion
                     (goto-char marker)
                     (end-of-line)
                     (insert " " (propertize "ok" 'face 'success)))))
             (error
              (with-current-buffer (rag-progress-buffer-get)
                (save-excursion
                  (goto-char marker)
                  (end-of-line)
                  (insert " " (propertize (apply #'format (cdr error-value))
                                          'face 'error))))))
         (kill-buffer (current-buffer))))
     '()
     t)))

(define-derived-mode rag-mode special-mode "RAG"
  "Rust news AGgragator."
  :interactive nil)

(provide 'rag)

;;; rag.el ends here
