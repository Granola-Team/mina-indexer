;; Ignore directories that don't contain our rust src
((nil . ((eval . (with-eval-after-load 'lsp-mode
                   (add-to-list 'lsp-file-watch-ignored-directories "[/\\\\]\\.cargo\\'")
                   (add-to-list 'lsp-file-watch-ignored-directories "[/\\\\]\\ops\\'")
                   (add-to-list 'lsp-file-watch-ignored-directories "[/\\\\]\\docs\\'"))))))
