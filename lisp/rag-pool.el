;;; rag-pool.el --- allocator pool -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Andrew Chi

;; Author: Andrew Chi <chifamicom@outlook.com>
;; Created: 28 Jun 2026
;; Version: 0.1.0
;; Keywords: data

;;; Commentary:

;;; Code:

(require 'rag-core)

(defvar rag-pool-allocators '()
  "A list of allocators.")

(defmacro rag-pool-with (var &rest body)
  "Run BODY with an allocator bound to VAR."
  (declare (indent 1))
  `(let ((,var (or (car-safe rag-pool-allocators)
                   (rag-core-bump-new))))
     (unwind-protect
         (progn
           (rag-core-bump-reset ,var)
           ,@body)
       (push ,var rag-pool-allocators))))

(provide 'rag-pool)

;;; rag-pool.el ends here
