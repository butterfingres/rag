pub mod fun {
    emacs::use_functions! {
        MAKE_VECTOR => "make-vector"
        REQUIRE => "require"
        SYMBOL_VALUE => "symbol-value"
    }
}
pub mod key {
    emacs::use_symbols! {
        TITLE => ":title"
        LINK => ":link"
        SKIP_DAYS => ":skip-days"
        SKIP_HOURS => ":skip-hours"
        TTL => ":ttl"
        LAST_UPDATE => ":last-update"

        DESCRIPTION => ":description"
        ID => ":id"
        PUB_DATE => ":pub-date"
        ENCLOSURES => ":enclosures"
    }
}
pub mod val {
    emacs::use_symbols! {
        MAKE_RAG_FEED => "make-rag-feed"
        MAKE_RAG_ENTRY => "make-rag-entry"
        RAG_ABI_VERSION => "rag-abi-version"
        RAG_LIB => "rag-lib"
    }
}
