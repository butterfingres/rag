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

/// Check if a feed should be fetched.
///
/// Returns non-nil when NOW is not in the cache period. TTL is the
/// `rag-entry-ttl' field, PERIOD is the `rag-entry-period' field,
/// LAST-UPDATE is the last update unix timestamp and NOW is the
/// current unix timestamp.
#[emacs::defun]
pub fn fetch_p(
    ttl: Option<String>,
    frequency: Option<i32>,
    last_update: Option<i64>,
    now: i64,
) -> Result<bool, emacs::Error> {
    if let Some(ttl) = ttl
        && let Some(last_update) = last_update
    {
        let ttl = Span::from_str(&ttl)?;
        let last_update = Timestamp::from_second(last_update)?;
        let now = Timestamp::from_second(now)?;

        let mut duration = ttl.to_duration(&last_update.to_zoned(TimeZone::UTC))?;
        if let Some(frequency) = frequency {
            duration = duration
                .checked_div(frequency)
                .ok_or(OverflowError(duration, frequency))?;
        }

        Ok(now > last_update.checked_add(duration)?)
    } else {
        Ok(true)
    }
}
