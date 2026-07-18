use {
    allocator_api2::{alloc::Allocator, boxed::Box},
    std::fmt::{self, Display, Formatter, Write as _},
};

pub enum Value<'a, A>
where
    A: Allocator,
{
    Nil,
    Cons(Box<Value<'a, A>, A>, Box<Value<'a, A>, A>),
    Char(char),
    String(&'a str),
}
impl<A> Display for Value<'_, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Cons(car, cdr) => write!(f, "({car} . {cdr})"),

            Self::Char('\u{7}') => f.write_str("?\\a"),
            Self::Char('\u{8}') => f.write_str("?\\b"),
            Self::Char('\t') => f.write_str("?\\t"),
            Self::Char('\n') => f.write_str("?\\n"),
            Self::Char('\u{b}') => f.write_str("?\\v"),
            Self::Char('\u{c}') => f.write_str("?\\f"),
            Self::Char('\r') => f.write_str("?\\r"),
            Self::Char('\u{1b}') => f.write_str("?\\e"),
            Self::Char(' ') => f.write_str("?\\s"),
            Self::Char('\u{7f}') => f.write_str("?\\d"),
            Self::Char(
                ch @ ('\\' | '\"' | '(' | ')' | '[' | ']' | ';' | '|' | '\'' | '`' | '#' | '.'
                | ','),
            ) => {
                write!(f, "?\\{ch}")
            }
            Self::Char(ch) => write!(f, "?{ch}"),

            Self::String(string) => {
                f.write_char('\"')?;
                for ch in string.chars() {
                    match ch {
                        '\u{7}' => f.write_str("\\a")?,
                        '\u{8}' => f.write_str("\\b")?,
                        '\t' => f.write_str("\\t")?,
                        '\u{b}' => f.write_str("\\v")?,
                        '\u{c}' => f.write_str("\\f")?,
                        '\r' => f.write_str("\\r")?,
                        '\u{1b}' => f.write_str("\\e")?,
                        '\\' | '\"' => write!(f, "\\{ch}")?,
                        '\u{7f}' => f.write_str("\\d")?,
                        _ => f.write_char(ch)?,
                    }
                }
                f.write_char('\"')?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::alloc::Dummy};

    macro_rules! test_value {
        ($ident:ident, $input:expr, $output:literal) => {
            #[test]
            fn $ident() -> ::std::result::Result<(), ::std::fmt::Error> {
                let mut buf = ::arrayvec::ArrayString::<{ $output.len() }>::new();
                ::std::write!(buf, "{}", $input)?;
                ::std::assert_eq!(buf.as_ref(), $output);
                ::std::result::Result::Ok(())
            }
        };
    }

    test_value!(value_display_nil, Value::<Dummy>::Nil, "nil");

    test_value!(
        value_display_char_control_g,
        Value::<Dummy>::Char('\u{7}'),
        "?\\a"
    );
    test_value!(
        value_display_char_backspace,
        Value::<Dummy>::Char('\u{8}'),
        "?\\b"
    );
    test_value!(value_display_char_tab, Value::<Dummy>::Char('\t'), "?\\t");
    test_value!(
        value_display_char_newline,
        Value::<Dummy>::Char('\n'),
        "?\\n"
    );
    test_value!(
        value_display_char_vertical_tab,
        Value::<Dummy>::Char('\u{b}'),
        "?\\v"
    );
    test_value!(
        value_display_char_form_feed,
        Value::<Dummy>::Char('\u{c}'),
        "?\\f"
    );
    test_value!(
        value_display_char_carriage_return,
        Value::<Dummy>::Char('\r'),
        "?\\r"
    );
    test_value!(
        value_display_char_escape,
        Value::<Dummy>::Char('\u{1b}'),
        "?\\e"
    );
    test_value!(value_display_char_space, Value::<Dummy>::Char(' '), "?\\s");
    test_value!(
        value_display_char_delete,
        Value::<Dummy>::Char('\u{7f}'),
        "?\\d"
    );
    test_value!(
        value_display_char_backslash,
        Value::<Dummy>::Char('\\'),
        "?\\\\"
    );
    test_value!(
        value_display_char_double_quote,
        Value::<Dummy>::Char('\"'),
        "?\\\""
    );
    test_value!(
        value_display_char_left_paren,
        Value::<Dummy>::Char('('),
        "?\\("
    );
    test_value!(
        value_display_char_right_paren,
        Value::<Dummy>::Char(')'),
        "?\\)"
    );
    test_value!(
        value_display_char_left_square_bracket,
        Value::<Dummy>::Char('['),
        "?\\["
    );
    test_value!(
        value_display_char_right_square_bracket,
        Value::<Dummy>::Char(']'),
        "?\\]"
    );
    test_value!(
        value_display_char_semicolon,
        Value::<Dummy>::Char(';'),
        "?\\;"
    );
    test_value!(value_display_char_pipe, Value::<Dummy>::Char('|'), "?\\|");
    test_value!(
        value_display_char_single_quote,
        Value::<Dummy>::Char('\''),
        "?\\\'"
    );
    test_value!(
        value_display_char_back_quote,
        Value::<Dummy>::Char('`'),
        "?\\`"
    );
    test_value!(value_display_char_hash, Value::<Dummy>::Char('#'), "?\\#");
    test_value!(value_display_char_period, Value::<Dummy>::Char('.'), "?\\.");
    test_value!(value_display_char_comma, Value::<Dummy>::Char(','), "?\\,");

    test_value!(value_display_char_regular, Value::<Dummy>::Char('a'), "?a");

    test_value!(
        value_display_string_hello_world,
        Value::<Dummy>::String("hello world"),
        "\"hello world\""
    );
    test_value!(
        value_display_string_control_g,
        Value::<Dummy>::String("\u{7}"),
        "\"\\a\""
    );
    test_value!(
        value_display_string_backspace,
        Value::<Dummy>::String("\u{8}"),
        "\"\\b\""
    );
    test_value!(
        value_display_string_tab,
        Value::<Dummy>::String("\t"),
        "\"\\t\""
    );
    test_value!(
        value_display_string_vertical_tab,
        Value::<Dummy>::String("\u{b}"),
        "\"\\v\""
    );
    test_value!(
        value_display_string_form_feed,
        Value::<Dummy>::String("\u{c}"),
        "\"\\f\""
    );
    test_value!(
        value_display_string_carriage_return,
        Value::<Dummy>::String("\r"),
        "\"\\r\""
    );
    test_value!(
        value_display_string_escape,
        Value::<Dummy>::String("\u{1b}"),
        "\"\\e\""
    );
    test_value!(
        value_display_string_backslash,
        Value::<Dummy>::String("\\"),
        "\"\\\\\""
    );
    test_value!(
        value_display_string_double_quote,
        Value::<Dummy>::String("\""),
        "\"\\\"\""
    );
    test_value!(
        value_display_string_delete,
        Value::<Dummy>::String("\u{7f}"),
        "\"\\d\""
    );
}
