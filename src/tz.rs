use jiff::tz::{TimeZone, offset};

pub const EDT: TimeZone = TimeZone::fixed(offset(-4));
