//! Feed utilities.

use {
    jiff::{SignedDuration, Span, Timestamp, tz::TimeZone},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        str::FromStr,
    },
};

#[derive(Debug)]
pub struct OverflowError(SignedDuration, i32);
impl Display for OverflowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "cannot divide {} by {}", self.0, self.1)
    }
}
impl Error for OverflowError {}

#[emacs::defun]
pub fn fetch_p(
    ttl: Option<String>,
    period: Option<i32>,
    last_update: i64,
    now: i64,
) -> Result<bool, emacs::Error> {
    if let Some(ttl) = ttl {
        let ttl = Span::from_str(&ttl)?;
        let last_update = Timestamp::from_second(last_update)?;
        let now = Timestamp::from_second(now)?;

        let mut duration = ttl.to_duration(&last_update.to_zoned(TimeZone::UTC))?;
        if let Some(period) = period {
            duration = duration
                .checked_div(period)
                .ok_or(OverflowError(duration, period))?;
        }

        Ok(now > last_update.checked_add(duration)?)
    } else {
        Ok(true)
    }
}
