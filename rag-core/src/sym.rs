pub mod f {
    //! Function symbols.

    emacs::use_functions! {
        SYMBOL_VALUE => "symbol-value"
    }
}
pub mod v {
    //! Other symbols.

    emacs::use_symbols! {
        RAG_DB_PATH => "rag-db-path"
        RAG_FEEDS => "rag-feeds"
    }
}
