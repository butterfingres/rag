pub mod f {
    //! Function symbols.

    emacs::use_functions! {
        REQUIRE => "require"
        SYMBOL_VALUE => "symbol-value"
    }
}
pub mod v {
    //! Other symbols.

    emacs::use_symbols! {
        RAG => "rag"
        RAG_DB_PATH => "rag-db-path"
        RAG_FEEDS => "rag-feeds"
    }
}
