use {
    allocator_api2::{alloc::Allocator, boxed::Box, vec::Vec},
    std::fmt::{self, Display, Formatter, Write as _},
};

pub enum Number {
    Signed(i64),
    Unsigned(u64),
}
impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Signed(num) => num.fmt(f),
            Self::Unsigned(num) => num.fmt(f),
        }
    }
}

pub enum Value<'a, A>
where
    A: Allocator,
{
    Nil,
    Cons(Box<Value<'a, A>, A>, Box<Value<'a, A>, A>),
    Char(char),
    List(Vec<Value<'a, A>, A>),
    Number(Number),
    String(&'a str),
    Symbol(&'a str),
}
impl<A> Display for Value<'_, A>
where
    A: Allocator,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Cons(car, cdr) => write!(f, "({car} . {cdr})"),
            Self::List(items) => {
                f.write_char('(')?;
                if let Some((car, cdr)) = items.split_first() {
                    car.fmt(f)?;
                    cdr.iter().try_for_each(|item| write!(f, " {item}"))?;
                }
                f.write_char(')')?;
                Ok(())
            }

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
                ch @ ('(' | ')' | '[' | ']' | '\\' | ';' | '\"' | '|' | '\'' | '`' | '#' | '.'
                | ','),
            ) => {
                write!(f, "?\\{ch}")
            }
            Self::Char(ch) => write!(f, "?{ch}"),

            Self::Number(num) => num.fmt(f),

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
            Self::Symbol("") => f.write_str("##"),
            Self::Symbol(symbol) => {
                for ch in symbol.chars() {
                    if let ' ' | '(' | ')' | '[' | ']' | '\\' | ';' | '\"' | '|' | '\'' | '`'
                    | '#' | '.' | ',' = ch
                    {
                        write!(f, "\\{ch}")?;
                    } else {
                        f.write_char(ch)?;
                    }
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::alloc::{Dummy, with_bump},
        allocator_api2::vec,
    };

    macro_rules! test_value {
        ($input:expr, $output:literal $(,)?) => {{
            let mut buf = ::arrayvec::ArrayString::<{ $output.len() }>::new();
            ::std::write!(buf, "{}", $input)?;
            ::std::assert_eq!(buf.as_ref(), $output);
            ::std::result::Result::Ok(())
        }};
        ($input:expr, $output:literal, $ident:ident $(,)?) => {
            #[test]
            fn $ident() -> ::std::result::Result<(), ::std::fmt::Error> {
                test_value!($input, $output)
            }
        };
    }

    #[test]
    fn value_display_cons() -> Result<(), fmt::Error> {
        with_bump(|bump| {
            test_value!(
                Value::Cons(
                    Box::new_in(Value::String("hello world"), &*bump),
                    Box::new_in(Value::Char('a'), &*bump)
                ),
                "(\"hello world\" . ?a)"
            )
        })
    }

    #[test]
    fn value_display_cons_list() -> Result<(), fmt::Error> {
        with_bump(|bump| {
            test_value!(
                Value::Cons(
                    Box::new_in(Value::Char('a'), &*bump),
                    Box::new_in(
                        Value::Cons(
                            Box::new_in(Value::Char('b'), &*bump),
                            Box::new_in(Value::Nil, &*bump)
                        ),
                        &*bump
                    )
                ),
                "(?a . (?b . nil))"
            )
        })
    }

    #[test]
    fn value_display_vector_list() -> Result<(), fmt::Error> {
        with_bump(|bump| {
            test_value!(
                Value::List(vec![in bump; Value::Char('a'), Value::Char('b')]),
                "(?a ?b)"
            )
        })
    }

    test_value!(Value::<Dummy>::Nil, "nil", value_display_nil);

    test_value!(
        Value::<Dummy>::Char('\u{7}'),
        "?\\a",
        value_display_char_control_g,
    );
    test_value!(
        Value::<Dummy>::Char('\u{8}'),
        "?\\b",
        value_display_char_backspace,
    );
    test_value!(Value::<Dummy>::Char('\t'), "?\\t", value_display_char_tab);
    test_value!(
        Value::<Dummy>::Char('\n'),
        "?\\n",
        value_display_char_newline,
    );
    test_value!(
        Value::<Dummy>::Char('\u{b}'),
        "?\\v",
        value_display_char_vertical_tab,
    );
    test_value!(
        Value::<Dummy>::Char('\u{c}'),
        "?\\f",
        value_display_char_form_feed,
    );
    test_value!(
        Value::<Dummy>::Char('\r'),
        "?\\r",
        value_display_char_carriage_return,
    );
    test_value!(
        Value::<Dummy>::Char('\u{1b}'),
        "?\\e",
        value_display_char_escape,
    );
    test_value!(Value::<Dummy>::Char(' '), "?\\s", value_display_char_space);
    test_value!(
        Value::<Dummy>::Char('\u{7f}'),
        "?\\d",
        value_display_char_delete,
    );
    test_value!(
        Value::<Dummy>::Char('\\'),
        "?\\\\",
        value_display_char_backslash,
    );
    test_value!(
        Value::<Dummy>::Char('\"'),
        "?\\\"",
        value_display_char_double_quote,
    );
    test_value!(
        Value::<Dummy>::Char('('),
        "?\\(",
        value_display_char_left_paren,
    );
    test_value!(
        Value::<Dummy>::Char(')'),
        "?\\)",
        value_display_char_right_paren,
    );
    test_value!(
        Value::<Dummy>::Char('['),
        "?\\[",
        value_display_char_left_square_bracket,
    );
    test_value!(
        Value::<Dummy>::Char(']'),
        "?\\]",
        value_display_char_right_square_bracket,
    );
    test_value!(
        Value::<Dummy>::Char(';'),
        "?\\;",
        value_display_char_semicolon,
    );
    test_value!(Value::<Dummy>::Char('|'), "?\\|", value_display_char_pipe);
    test_value!(
        Value::<Dummy>::Char('\''),
        "?\\\'",
        value_display_char_single_quote,
    );
    test_value!(
        Value::<Dummy>::Char('`'),
        "?\\`",
        value_display_char_back_quote,
    );
    test_value!(Value::<Dummy>::Char('#'), "?\\#", value_display_char_hash);
    test_value!(Value::<Dummy>::Char('.'), "?\\.", value_display_char_period);
    test_value!(Value::<Dummy>::Char(','), "?\\,", value_display_char_comma);

    test_value!(Value::<Dummy>::Char('a'), "?a", value_display_char_regular);

    test_value!(
        Value::<Dummy>::String("hello world"),
        "\"hello world\"",
        value_display_string_hello_world,
    );
    test_value!(
        Value::<Dummy>::String("\u{7}"),
        "\"\\a\"",
        value_display_string_control_g,
    );
    test_value!(
        Value::<Dummy>::String("\u{8}"),
        "\"\\b\"",
        value_display_string_backspace,
    );
    test_value!(
        Value::<Dummy>::String("\t"),
        "\"\\t\"",
        value_display_string_tab,
    );
    test_value!(
        Value::<Dummy>::String("\u{b}"),
        "\"\\v\"",
        value_display_string_vertical_tab,
    );
    test_value!(
        Value::<Dummy>::String("\u{c}"),
        "\"\\f\"",
        value_display_string_form_feed,
    );
    test_value!(
        Value::<Dummy>::String("\r"),
        "\"\\r\"",
        value_display_string_carriage_return,
    );
    test_value!(
        Value::<Dummy>::String("\u{1b}"),
        "\"\\e\"",
        value_display_string_escape,
    );
    test_value!(
        Value::<Dummy>::String("\\"),
        "\"\\\\\"",
        value_display_string_backslash,
    );
    test_value!(
        Value::<Dummy>::String("\""),
        "\"\\\"\"",
        value_display_string_double_quote,
    );
    test_value!(
        Value::<Dummy>::String("\u{7f}"),
        "\"\\d\"",
        value_display_string_delete,
    );

    test_value!(Value::<Dummy>::Symbol(""), "##", value_display_symbol_empty);
    test_value!(
        Value::<Dummy>::Symbol("hello-world"),
        "hello-world",
        value_display_symbol_hello_world
    );

    test_value!(
        Value::<Dummy>::Symbol(" "),
        "\\ ",
        value_display_symbol_space
    );
    test_value!(
        Value::<Dummy>::Symbol("("),
        "\\(",
        value_display_symbol_left_paren
    );
    test_value!(
        Value::<Dummy>::Symbol(")"),
        "\\)",
        value_display_symbol_right_paren
    );
    test_value!(
        Value::<Dummy>::Symbol("["),
        "\\[",
        value_display_symbol_left_square_bracket
    );
    test_value!(
        Value::<Dummy>::Symbol("]"),
        "\\]",
        value_display_symbol_right_square_bracket
    );
    test_value!(
        Value::<Dummy>::Symbol("\\"),
        "\\\\",
        value_display_symbol_backslash
    );
    test_value!(
        Value::<Dummy>::Symbol(";"),
        "\\;",
        value_display_symbol_semicolon
    );
    test_value!(
        Value::<Dummy>::Symbol("\""),
        "\\\"",
        value_display_symbol_double_quote
    );
    test_value!(
        Value::<Dummy>::Symbol("|"),
        "\\|",
        value_display_symbol_pipe,
    );
    test_value!(
        Value::<Dummy>::Symbol("\'"),
        "\\\'",
        value_display_symbol_quote
    );
    test_value!(
        Value::<Dummy>::Symbol("`"),
        "\\`",
        value_display_symbol_backquote
    );
    test_value!(
        Value::<Dummy>::Symbol("#"),
        "\\#",
        value_display_symbol_hash
    );
    test_value!(
        Value::<Dummy>::Symbol("."),
        "\\.",
        value_display_symbol_period
    );
    test_value!(
        Value::<Dummy>::Symbol(","),
        "\\,",
        value_display_symbol_comma
    );

    test_value!(
        Value::<Dummy>::Number(Number::Signed(-10)),
        "-10",
        value_display_number_signed
    );
    test_value!(
        Value::<Dummy>::Number(Number::Unsigned(10)),
        "10",
        value_display_number_unsigned
    );
}
