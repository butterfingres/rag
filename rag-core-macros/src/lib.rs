use {
    proc_macro::TokenStream,
    proc_macro2::Span,
    syn::{
        Data, DataStruct, DeriveInput, Fields, FieldsNamed, parse_macro_input, spanned::Spanned,
    },
};

#[proc_macro_derive(ParseXml, attributes(select, when, repeat))]
pub fn parse_xml(input: TokenStream) -> TokenStream {
    let data = parse_macro_input!(input as DeriveInput);
    let span = data.span();
    parse_xml_inner(data, span).unwrap_or_else(|e| e.into_compile_error().into())
}

fn parse_xml_inner(
    DeriveInput { data, .. }: DeriveInput,
    span: Span,
) -> Result<TokenStream, syn::Error> {
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named: _fields, .. }),
        ..
    }) = data
    else {
        return Err(syn::Error::new(
            span,
            "cannot derive on types that are not structs with named fields",
        ));
    };

    todo!()
}

// struct Foo {
//     #[tag("foo")]
//     #[attr("foo")]
//     #[attr_eq("foo", "bar")]
//     title: Option<Box<str>>,
//     #[tag("foo")]
//     #[or]
//     #[tag("bar")]
//     #[priority(1)]
//     title: Option<Box<str>>,
// }

// PREDICATE = TAG |
//             [ATTR] |
//             [ATTR] == STRING |
//             PREDICATE && PREDICATE |
//             PREDICATE || PREDICATE |
//             (PREDICATE)
// ACTION = Authority::Strong |
//          Authority::Weak
//
// attributes: select, when, repeat
