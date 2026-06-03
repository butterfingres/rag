use {
    crate::{
        split::{AsciiSplitter, Split},
        tz,
    },
    jiff::{
        ToSpan, Zoned,
        civil::Date,
        fmt::temporal::DateTimeParser,
        tz::{Offset, TimeZone, offset},
    },
    std::{
        fmt::{self, Display, Formatter},
        num::ParseIntError,
        str::FromStr,
    },
};

#[derive(Debug)]
pub enum Section {
    Weekday,
    Day,
    Month,
    Year,
    Time,
    Timezone,
}
impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Self::Weekday => "weekday",
            Self::Day => "day",
            Self::Month => "month",
            Self::Year => "year",
            Self::Time => "time",
            Self::Timezone => "timezone",
        })
    }
}

#[derive(Debug)]
pub enum Error {
    ShortOffset,
    MissingSection(Section),
    ParseInt(ParseIntError),
    Time(jiff::Error),
    UnknownMonth(Box<str>),
    UnknownTimezone(Box<str>),
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::ShortOffset => f.write_str("offset is too short"),
            Self::MissingSection(section) => {
                write!(f, "rfc 822 timestamp is missing a {section} section")
            }
            Self::ParseInt(e) => e.fmt(f),
            Self::Time(e) => e.fmt(f),
            Self::UnknownMonth(month) => write!(f, "unknown month {month}"),
            Self::UnknownTimezone(tz) => write!(f, "unknown timezone {tz}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
impl From<jiff::Error> for Error {
    fn from(e: jiff::Error) -> Self {
        Self::Time(e)
    }
}

pub fn parse(dt: &str) -> Result<Zoned, Error> {
    let mut words = AsciiSplitter::<b' '>::split(dt);

    // we probably don't need to check
    let weekday = words
        .next()
        .ok_or(Error::MissingSection(Section::Weekday))?;

    let day = if !weekday.ends_with(',') {
        weekday
    } else {
        words.next().ok_or(Error::MissingSection(Section::Day))?
    };
    let day = i8::from_str(day)?;

    let month = words
        .next()
        .ok_or(Error::MissingSection(Section::Month))
        .and_then(|month| match month {
            "Jan" => Ok(1),
            "Feb" => Ok(2),
            "Mar" => Ok(3),
            "Apr" => Ok(4),
            "May" => Ok(5),
            "Jun" => Ok(6),
            "Jul" => Ok(7),
            "Aug" => Ok(8),
            "Sep" => Ok(9),
            "Oct" => Ok(10),
            "Nov" => Ok(11),
            "Dec" => Ok(12),
            _ => Err(Error::UnknownMonth(Box::from(month))),
        })?;

    let year = words.next().ok_or(Error::MissingSection(Section::Year))?;
    let digits = year.len();
    let mut year = i16::from_str(year)?;

    if digits == 2 {
        let century = if year < 50 { 2000 } else { 1900 };
        year += century;
    }
    let year = year;

    let time = DateTimeParser::new()
        .parse_time(words.next().ok_or(Error::MissingSection(Section::Time))?)?;

    let offset = words
        .next()
        .ok_or(Error::MissingSection(Section::Timezone))
        .and_then(|tz| {
            if let Some((negate, offset)) = {
                let mut chrs = tz.chars();
                chrs.next()
                    .map(|ch| (ch, chrs.as_str()))
                    .and_then(|(ch, offset)| match ch {
                        '+' => Some((false, offset)),
                        '-' => Some((true, offset)),
                        _ => None,
                    })
            } {
                let (hour, mins) = offset.split_at_checked(2).ok_or(Error::ShortOffset)?;
                let hours = i8::from_str(hour)?;
                let mins = i8::from_str(mins)?;

                let mut offset = Offset::from_hours(hours)?.saturating_add(mins.minutes());
                if negate {
                    offset = offset.negate();
                }

                Ok(offset)
            } else {
                match tz {
                    "UT" | "GMT" => Ok(tz::GMT),
                    "EST" => Ok(tz::EST),
                    "EDT" => Ok(tz::EDT),
                    "CST" => Ok(tz::CST),
                    "CDT" => Ok(tz::CDT),
                    "MST" => Ok(tz::MST),
                    "MDT" => Ok(tz::MDT),
                    "PST" => Ok(tz::PST),
                    "PDT" => Ok(tz::PDT),
                    "Z" => Ok(tz::Z),
                    "A" => Ok(tz::A),
                    "M" => Ok(tz::M),
                    "N" => Ok(tz::N),
                    "Y" => Ok(tz::Y),
                    tz => Err(Error::UnknownTimezone(Box::from(tz))),
                }
            }
        })?;

    Date::new(year, month, day)?
        .to_datetime(time)
        .to_zoned(TimeZone::fixed(offset))
        .map_err(Error::Time)
}

#[cfg(test)]
mod tests {
    use {super::*, jiff::civil::DateTime};

    #[test]
    fn test_parse() -> Result<(), Error> {
        [
            (
                ["Wed, 02 Oct 2002 08:00:00 EST", "02 Oct 2002 08:00:00 EST"],
                DateTime::new(2002, 10, 02, 08, 00, 00, 00)?.to_zoned(TimeZone::fixed(tz::EST))?,
            ),
            (
                ["Wed, 02 Oct 2002 13:00:00 GMT", "02 Oct 2002 13:00:00 GMT"],
                DateTime::new(2002, 10, 02, 13, 00, 00, 00)?.to_zoned(TimeZone::fixed(tz::GMT))?,
            ),
            (
                [
                    "Wed, 02 Oct 2002 15:00:00 +0200",
                    "02 Oct 2002 15:00:00 +0200",
                ],
                DateTime::new(2002, 10, 02, 15, 00, 00, 00)?
                    .to_zoned(TimeZone::fixed(offset(2)))?,
            ),
            (
                ["Wed, 02 Oct 2002 15:00:00 A", "02 Oct 2002 15:00:00 A"],
                DateTime::new(2002, 10, 02, 15, 00, 00, 00)?.to_zoned(TimeZone::fixed(tz::A))?,
            ),
        ]
        .into_iter()
        .try_for_each(|(inputs, output)| {
            inputs.into_iter().try_for_each(|input| {
                assert_eq!(parse(input)?, output);
                Ok(())
            })
        })
    }
}
