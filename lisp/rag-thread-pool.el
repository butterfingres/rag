;;; rag-thread-pool.el --- rust thread pool -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 19 Jul 2026
;; Version: 0.1.0
;; Keywords: data, lisp

;;; Commentary:

;;; Code:

(require 'rag-core)

(defgroup rag-thread-pool '()
  "Rust thread pool."
  :group 'rag)

(defcustom rag-thread-pool-log-buffer-name " *rag-thread-pool-log*"
  "The buffer name for thread pool errors."
  :group 'rag-thread-pool
  :type 'string)

(defvar rag-thread-pool nil
  "A thread pool.")

(defun rag-thread-pool-get ()
  "Get the thread pool object."
  (declare (ftype (function () user-ptr)))
  (with-memoization rag-thread-pool
    (let ((process (make-pipe-process :name "rag-thread-pool-log"
                                      :buffer (get-buffer-create rag-thread-pool-log-buffer-name))))
      (rag-core-thread-pool-new process))))

(provide 'rag-thread-pool)

;;; rag-thread-pool.el ends here
