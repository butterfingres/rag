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

fn fetch_p_inner<T>(
    ttl: Option<T>,
    frequency: Option<i32>,
    last_update: Option<i64>,
    skip_hours: u32,
    skip_days: u8,
    now: i64,
) -> Result<bool, emacs::Error>
where
    T: AsRef<str>,
{
    if let Some(ttl) = ttl
        && let Some(last_update) = last_update
    {
        let ttl = Span::from_str(ttl.as_ref())?;
        let last_update = Timestamp::from_second(last_update)?;

        let now = Timestamp::from_second(now)?;

        let skip_days = SkipDays::new([skip_days]);
        let skip_hours = SkipHours::new([skip_hours]);
        let zoned = now.to_zoned(tz::GMT);
        if usize::try_from(zoned.hour())
            .map(|idx| skip_hours[idx])
            .unwrap_or_default()
            || usize::try_from(zoned.weekday().to_monday_zero_offset())
                .map(|idx| skip_days[idx])
                .unwrap_or_default()
        {
            return Ok(false);
        }

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
#[emacs::defun]
pub fn fetch_p(
    ttl: Option<String>,
    frequency: Option<i32>,
    last_update: Option<i64>,
    skip_days: u32,
    skip_hours: u8,
    now: i64,
) -> Result<bool, emacs::Error> {
    fetch_p_inner(ttl, frequency, last_update, skip_days, skip_hours, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_p_empty() -> Result<(), emacs::Error> {
        assert!(
            fetch_p_inner(None::<&str>, None, None, 0, 0, 0)?,
            "empty feeds should always be fetched"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_below_cache() -> Result<(), emacs::Error> {
        assert!(
            !fetch_p_inner(Some("PT10M"), None, Some(0), 0, 0, 0)?,
            "0 < 0 + 10"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_default() -> Result<(), emacs::Error> {
        assert!(fetch_p_inner(Some("PT1S"), None, None, 0, 0, 1)?, "1 > 0");
        Ok(())
    }

    #[test]
    fn test_fetch_p_last_update() -> Result<(), emacs::Error> {
        assert!(
            fetch_p_inner(Some("PT1S"), None, Some(1), 0, 0, 2)?,
            "2 >= 1 + 1"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_frequency() -> Result<(), emacs::Error> {
        assert!(
            fetch_p_inner(Some("PT1M"), Some(2), Some(0), 0, 0, 30)?,
            "60 / 2 >= 30"
        );
        Ok(())
    }

    #[test]
    fn test_fetch_p_no_panic() -> Result<(), emacs::Error> {
        assert!(
            fetch_p_inner(Some("PT1M"), Some(0), Some(0), 0, 0, 30).is_err(),
            "this should not panic"
        );
        Ok(())
    }
}
