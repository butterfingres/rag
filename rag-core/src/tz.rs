use jiff::tz::{Offset, offset};

pub const GMT: Offset = offset(0);
pub const EST: Offset = offset(-5);
pub const EDT: Offset = offset(-4);
pub const CST: Offset = offset(-6);
pub const CDT: Offset = offset(-5);
pub const MST: Offset = offset(-7);
pub const MDT: Offset = offset(-6);
pub const PST: Offset = offset(-8);
pub const PDT: Offset = offset(-7);
pub const Z: Offset = offset(0);
pub const A: Offset = offset(-1);
pub const M: Offset = offset(-12);
pub const N: Offset = offset(1);
pub const Y: Offset = offset(12);
