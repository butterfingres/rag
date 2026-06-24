use jiff::tz::{TimeZone, offset};

pub const Z: TimeZone = TimeZone::fixed(offset(0));
pub const GMT: TimeZone = TimeZone::fixed(offset(0));
pub const EDT: TimeZone = TimeZone::fixed(offset(-4));
