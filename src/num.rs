use std::fmt::{self, Display, Formatter};

pub trait UnsignedInteger: From<u8> + PartialOrd {
    const ZERO: Self;
    const TEN: Self;

    fn checked_add(self, r: Self) -> Option<Self>;
    fn checked_mul(self, r: Self) -> Option<Self>;
}

macro_rules! impl_uint {
    ($ty:ty) => {
        impl $crate::num::UnsignedInteger for $ty {
            const ZERO: Self = 0;
            const TEN: Self = 10;

            fn checked_add(self, r: Self) -> Option<Self> {
                self.checked_add(r)
            }
            fn checked_mul(self, r: Self) -> Option<Self> {
                self.checked_mul(r)
            }
        }
    };
}
impl_uint!(u8);
impl_uint!(u32);
impl_uint!(u64);

#[derive(Debug)]
pub enum ParseIntError {
    UnknownDigit,
    Overflow,
}
impl Display for ParseIntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::UnknownDigit => f.write_str("number was not in base 10"),
            Self::Overflow => f.write_str("number was too large"),
        }
    }
}
pub fn parse<T>(bytes: &[u8]) -> Result<T, ParseIntError>
where
    T: UnsignedInteger,
{
    let mut num = T::ZERO;

    for digit in bytes
        .iter()
        .map(|b| b.wrapping_sub(b'0'))
        .map(T::from)
        .skip_while(|digit| *digit == T::ZERO)
    {
        if digit > T::from(9) {
            return Err(ParseIntError::UnknownDigit);
        }
        num = num
            .checked_mul(T::TEN)
            .and_then(|num| num.checked_add(digit))
            .ok_or(ParseIntError::Overflow)?;
    }

    Ok(num)
}

#[cfg(test)]
mod tests {
    use {super::*, std::assert_matches};

    #[test]
    fn test_parse() {
        assert_matches!(parse::<u8>(b"10"), Ok(10));
        assert_matches!(parse::<u8>(b"1000"), Err(ParseIntError::Overflow));
        assert_matches!(parse::<u32>(b"123456789"), Ok(123456789));
        assert_matches!(parse::<u8>(b"asdf"), Err(ParseIntError::UnknownDigit));
    }
}
