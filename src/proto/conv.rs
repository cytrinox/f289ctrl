use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

pub fn timestamp_to_datetime(ts: f64) -> DateTime<Utc> {
    // Timestamp is in local time, but we can't construct it directly.
    // Let's first assume UTC, then fake the timezone to local.
    let naive = Utc.timestamp_nanos((ts * 1000000000.0) as i64).naive_utc();
    Local.from_local_datetime(&naive).unwrap().into()
}

pub type DeviceDateTime = NaiveDateTime;

pub fn unit_prefix(unit_multiplier: i16) -> &'static str {
    match unit_multiplier {
        -12 => "p",
        -9 => "n",
        -6 => "u",
        -3 => "m",
        -2 => "c",
        -1 => "d",
        0 => "",
        1 => "D",
        2 => "h",
        3 => "k",
        6 => "M",
        9 => "G",
        12 => "T",
        _ => "?",
    }
}

pub fn pretty_ts(&ts: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = ts.into();
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}
