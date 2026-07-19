use std::fmt::{self, Display, Formatter, Write as _};

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

#[derive(Default)]
pub enum Value<'a> {
    #[default]
    Nil,
    Number(Number),
    String(&'a str),
}
impl Display for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Nil => f.write_str("nil"),

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
        }
    }
}

pub fn fmt_vector<'a, I, T>(iter: I, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<Result<Value<'a>, fmt::Error>>,
{
    fn inner<'a>(
        iter: &mut dyn Iterator<Item = Result<Value<'a>, fmt::Error>>,
        f: &mut Formatter<'_>,
    ) -> Result<(), fmt::Error> {
        f.write_char('[')?;
        if let Some(item) = iter.next() {
            let item = item?;
            item.fmt(f)?;

            for item in iter {
                let item = item?;
                write!(f, " {item}")?;
            }
        }
        f.write_char(']')
    }

    inner(
        &mut iter
            .into_iter()
            .map(<T as Into<Result<Value<'a>, fmt::Error>>>::into),
        f,
    )
}

#[cfg(test)]
mod tests {
    use {super::*, arrayvec::ArrayString};

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
    fn value_display_list() -> Result<(), fmt::Error> {
        const OUTPUT: &str = "[1 2]";
        let mut buf = ArrayString::<{ OUTPUT.len() }>::new();
        write!(
            buf,
            "{}",
            fmt::from_fn(|f| fmt_vector(
                [1, 2].map(Number::Unsigned).map(Value::Number).map(Ok),
                f
            ))
        )?;
        assert_eq!(buf.as_ref(), OUTPUT);
        Ok(())
    }

    test_value!(Value::Nil, "nil", value_display_nil);

    test_value!(
        Value::String("hello world"),
        "\"hello world\"",
        value_display_string_hello_world,
    );
    test_value!(
        Value::String("\u{7}"),
        "\"\\a\"",
        value_display_string_control_g,
    );
    test_value!(
        Value::String("\u{8}"),
        "\"\\b\"",
        value_display_string_backspace,
    );
    test_value!(Value::String("\t"), "\"\\t\"", value_display_string_tab,);
    test_value!(
        Value::String("\u{b}"),
        "\"\\v\"",
        value_display_string_vertical_tab,
    );
    test_value!(
        Value::String("\u{c}"),
        "\"\\f\"",
        value_display_string_form_feed,
    );
    test_value!(
        Value::String("\r"),
        "\"\\r\"",
        value_display_string_carriage_return,
    );
    test_value!(
        Value::String("\u{1b}"),
        "\"\\e\"",
        value_display_string_escape,
    );
    test_value!(
        Value::String("\\"),
        "\"\\\\\"",
        value_display_string_backslash,
    );
    test_value!(
        Value::String("\""),
        "\"\\\"\"",
        value_display_string_double_quote,
    );
    test_value!(
        Value::String("\u{7f}"),
        "\"\\d\"",
        value_display_string_delete,
    );

    test_value!(
        Value::Number(Number::Signed(-10)),
        "-10",
        value_display_number_signed
    );
    test_value!(
        Value::Number(Number::Unsigned(10)),
        "10",
        value_display_number_unsigned
    );
}
