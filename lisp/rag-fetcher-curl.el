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

(defcustom rag-fetcher-curl-switches '("--silent" "--show-error"
                                       "--fail")
  "Parameters to pass before the url.

This is always given to the process and cannot be overwritten."
  :group 'rag-fetcher-curl
  :type '(repeat string))

(cl-defstruct rag-fetcher-curl
  "Curl fetcher configuration."
  program
  switches)

(cl-defmethod rag-fetcher-fetch-url (backend-config
                                     (rag-fetcher-curl curl-config)
                                     url
                                     callback)
  (let ((buffer (generate-new-buffer " *rag-fetcher-curl-temp*")))
    (condition-case _error
        (make-process :name "curl"
                      :buffer buffer
                      :command (append (or (rag-fetcher-curl-program curl-config)
                                           rag-fetcher-curl-program)
                                       rag-fetcher-curl-switches
                                       (rag-fetcher-curl-switches curl-config)
                                       (when-let* ((timeout (rag-fetcher-config-timeout backend-config)))
                                         `("--max-time" ,(number-to-string timeout)))
                                       (list url))
                      :sentinel (lambda (process _status)
                                  (when (memq (process-status process) '(exit signal))
                                    (unwind-protect
                                        (with-current-buffer buffer
                                          (funcall callback))
                                      (kill-buffer buffer)))))
      (error (kill-buffer buffer)))))

(provide 'rag-fetcher-curl)

;;; rag-fetcher-curl.el ends here
