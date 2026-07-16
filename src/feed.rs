//! Feed utilities.

use {
    crate::{
        tz,
        xml::{SkipDays, SkipHours},
    },
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

fn fetch_p_inner(
    ttl: Option<&str>,
    frequency: Option<i32>,
    last_update: Option<i64>,
    skip_hours: Option<u32>,
    skip_days: Option<u8>,
    now: i64,
) -> Result<bool, rem::Error> {
    let now = Timestamp::from_second(now)?;

    let zoned = now.to_zoned(tz::GMT);
    if let Some(skip_hours) = skip_hours
        && usize::try_from(zoned.hour())
            .map(|idx| SkipHours::new([skip_hours])[idx])
            .unwrap_or_default()
    {
        return Ok(false);
    }
    if let Some(skip_days) = skip_days
        && usize::try_from(zoned.weekday().to_monday_zero_offset())
            .map(|idx| SkipDays::new([skip_days])[idx])
            .unwrap_or_default()
    {
        return Ok(false);
    }

    if let Some(ttl) = ttl
        && let Some(last_update) = last_update
    {
        let ttl = Span::from_str(ttl)?;
        let last_update = Timestamp::from_second(last_update)?;

        let mut duration = ttl.to_duration(&last_update.to_zoned(TimeZone::UTC))?;
        if let Some(frequency) = frequency {
            duration = duration
                .checked_div(frequency)
                .ok_or(OverflowError(duration, frequency))?;
        }

        Ok(now >= last_update.checked_add(duration)?)
    } else {
        Ok(true)
    }
}

/// Check if a feed should be fetched.
///
/// Returns non-nil when NOW is not in the cache period. TTL is the
/// `rag-entry-ttl' field, PERIOD is the `rag-entry-period' field,
/// LAST-UPDATE is the last update unix timestamp and NOW is the
/// current unix timestamp.
#[rem::defun]
pub fn fetch_p(
    ttl: Option<String>,
    frequency: Option<i32>,
    last_update: Option<i64>,
    skip_days: Option<u32>,
    skip_hours: Option<u8>,
    now: i64,
) -> Result<bool, rem::Error> {
    fetch_p_inner(
        ttl.as_deref(),
        frequency,
        last_update,
        skip_days,
        skip_hours,
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_p_empty() -> Result<(), rem::Error> {
        assert!(
            fetch_p_inner(None::<&str>, None, None, None, None, 0)?,
            "empty feeds should always be fetched"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_below_cache() -> Result<(), rem::Error> {
        assert!(
            !fetch_p_inner(Some("PT10M"), None, Some(0), None, None, 0)?,
            "0 < 0 + 10"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_default() -> Result<(), rem::Error> {
        assert!(
            fetch_p_inner(Some("PT1S"), None, None, None, None, 1)?,
            "1 > 0"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_last_update() -> Result<(), rem::Error> {
        assert!(
            fetch_p_inner(Some("PT1S"), None, Some(1), None, None, 2)?,
            "2 >= 1 + 1"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_frequency() -> Result<(), rem::Error> {
        assert!(
            fetch_p_inner(Some("PT1M"), Some(2), Some(0), None, None, 30)?,
            "60 / 2 >= 30"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_no_panic() {
        assert!(
            fetch_p_inner(Some("PT1M"), Some(0), Some(0), None, None, 30).is_err(),
            "this should not panic"
        );
    }

    #[test]
    fn test_fetch_p_skip_hours() -> Result<(), rem::Error> {
        assert!(
            !fetch_p_inner(
                None::<&str>,
                None,
                Some(0),
                Some(0b0000_0000_0000_0000_0000_0000_0000_0001),
                None,
                0,
            )?,
            "the zeroth hour is skipped"
        );
        assert!(
            !fetch_p_inner(
                None::<&str>,
                None,
                Some(0),
                Some(0b0000_0000_0000_0000_0000_0000_0000_0010),
                None,
                60 * 60,
            )?,
            "the first hour is skipped"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_skip_days() -> Result<(), rem::Error> {
        assert!(
            !fetch_p_inner(None::<&str>, None, Some(0), None, Some(0b0000_1000), 0)?,
            "January 1 1970 is in Thursday"
        );
        assert!(
            !fetch_p_inner(
                None::<&str>,
                None,
                Some(0),
                None,
                Some(0b0001_0000),
                60 * 60 * 24
            )?,
            "January 2 1970 is in Friday"
        );
        Ok(())
    }
}
