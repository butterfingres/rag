use chrono::{DateTime, FixedOffset};

pub enum Skip {
    Hour(u8),
    Weekday(u8),
}

pub enum UpdatePeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}
pub struct Update {
    pub period: UpdatePeriod,
    pub frequency: u32,
    pub base: DateTime<FixedOffset>,
}

pub struct PartialFeed<T> {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: Option<DateTime<FixedOffset>>,
    /// Extra metadata to add to the feed.
    meta: T,
}
pub struct Feed {
    pub title: Box<str>,
    // The link is optional in atom.
    pub link: Option<Box<str>>,
    pub skips: Vec<Skip>,
    pub update: Option<Update>,
    pub last_update: DateTime<FixedOffset>,
}

pub struct PartialEntry<T> {
    pub title: Option<Box<str>>,
    pub link: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub pub_date: Option<DateTime<FixedOffset>>,
    pub fetch_date: DateTime<FixedOffset>,
    pub enclosures: Vec<Box<str>>,
    meta: T,
}
pub type Entry = PartialEntry<()>;
