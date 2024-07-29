use chrono::{DateTime, Datelike, TimeZone, Utc};

use crate::habit::Day;

pub fn left_pad(s: &str, c: char, n: usize) -> String {
    if n <= s.len() {
        return String::from(s);
    }

    let mut new_s = String::from("");
    let n_it = n - s.len();
    for _ in 0..n_it {
        new_s.push(c);
    }

    new_s.push_str(s);
    new_s
}

pub fn display_days(days: &[Day]) -> String {
    if days.is_empty() {
        return String::from("");
    }

    let mut res = String::from("");
    for day in days.iter().take(days.len() - 2) {
        res.push_str(&day.to_string());
        res.push_str(", ");
    }
    res.push_str(&days[days.len() - 2].to_string());
    res.push_str(" and ");
    res.push_str(&days[days.len() - 1].to_string());

    res
}

pub fn nth_day_of_year(datetime: &DateTime<Utc>) -> u16 {
    let first_day_of_year = Utc
        .with_ymd_and_hms(datetime.year(), 1, 1, 0, 0, 0)
        .unwrap();

    (datetime.signed_duration_since(first_day_of_year).num_days() + 1) as u16
}
