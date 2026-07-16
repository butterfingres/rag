pub mod fun {
    rem::use_functions! {
        MAKE_VECTOR => "make-vector",
        REQUIRE => "require",
        SYMBOL_VALUE => "symbol-value",
        STRING_BYTES => "string-bytes",
    }
}
pub mod key {
    rem::use_symbols! {
        TITLE => ":title",
        LINK => ":link",
        SKIP_DAYS => ":skip-days",
        SKIP_HOURS => ":skip-hours",
        TTL => ":ttl",
        FREQUENCY => ":frequency",
        LAST_UPDATE => ":last-update",

        DESCRIPTION => ":description",
        ID => ":id",
        PUB_DATE => ":pub-date",
        ENCLOSURES => ":enclosures",
    }
}
pub mod val {
    rem::use_symbols! {
        MAKE_RAG_FEED => "make-rag-feed",
        MAKE_RAG_ENTRY => "make-rag-entry",
        RAG_ABI_VERSION => "rag-abi-version",
        RAG_LIB => "rag-lib",
    }
}
