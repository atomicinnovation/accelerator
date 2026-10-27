//! When a work item was created, as its `date` field records it.

use std::cmp::Ordering;

use crate::show::read_field_raw;
use crate::work_item_files::frontmatter_of;
use crate::work_item_files::WorkItemFile;

const SECONDS_PER_DAY: i64 = 86_400;

/// Seconds since the Unix epoch in UTC, or `None` when the item records no
/// date this can read. An undated item orders after every dated one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationDate(Option<i64>);

impl CreationDate {
    #[must_use]
    pub fn of(file: &WorkItemFile) -> Self {
        Self(
            frontmatter_of(&file.content)
                .and_then(|frontmatter| read_field_raw(frontmatter, "date"))
                .and_then(|value| seconds_since_epoch(&value)),
        )
    }
}

impl Ord for CreationDate {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0, other.0) {
            (Some(mine), Some(theirs)) => mine.cmp(&theirs),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

impl PartialOrd for CreationDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Reads `YYYY-MM-DD` as midnight UTC, and `YYYY-MM-DDTHH:MM:SS` as UTC
/// unless it carries an offset.
fn seconds_since_epoch(value: &str) -> Option<i64> {
    let value = value.trim();
    let (date, time) = match value.get(10..11) {
        None => (value, None),
        Some("T" | "t" | " ") => (&value[..10], Some(&value[11..])),
        Some(_) => return None,
    };
    let day = days_since_epoch(date)?;
    let seconds = time.map_or(Some(0), seconds_into_day)?;
    Some(day * SECONDS_PER_DAY + seconds)
}

fn days_since_epoch(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year = fixed_digits(parts.next()?, 4)?;
    let month = fixed_digits(parts.next()?, 2)?;
    let day = fixed_digits(parts.next()?, 2)?;
    if parts.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
    {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn seconds_into_day(time: &str) -> Option<i64> {
    let (clock, offset) = split_offset(time)?;
    let mut parts = clock.split(':');
    let hour = fixed_digits(parts.next()?, 2)?;
    let minute = fixed_digits(parts.next()?, 2)?;
    let second = fixed_digits(parts.next()?, 2)?;
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(hour * 3_600 + minute * 60 + second - offset)
}

/// The clock part and the offset east of UTC in seconds.
fn split_offset(time: &str) -> Option<(&str, i64)> {
    if let Some(clock) = time.strip_suffix(['Z', 'z']) {
        return Some((clock, 0));
    }
    let Some(sign_at) = time.rfind(['+', '-']) else {
        return Some((time, 0));
    };
    let (clock, offset) = time.split_at(sign_at);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let (hours, minutes) = offset[1..].split_once(':')?;
    let hours = fixed_digits(hours, 2)?;
    let minutes = fixed_digits(minutes, 2)?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some((clock, sign * (hours * 3_600 + minutes * 60)))
}

fn fixed_digits(text: &str, width: usize) -> Option<i64> {
    (text.len() == width && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Howard Hinnant's `days_from_civil`: days since 1970-01-01 in the
/// proleptic Gregorian calendar.
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CreationDate;
    use crate::work_item_files::WorkItemFile;

    fn dated(value: &str) -> CreationDate {
        CreationDate::of(&WorkItemFile {
            path: PathBuf::from("meta/work/0001-item.md"),
            content: format!("---\nid: \"0001\"\ndate: \"{value}\"\n---\n"),
        })
    }

    fn undated() -> CreationDate {
        CreationDate::of(&WorkItemFile {
            path: PathBuf::from("meta/work/0001-item.md"),
            content: "---\nid: \"0001\"\n---\n".to_owned(),
        })
    }

    #[test]
    fn a_date_only_value_orders_as_midnight_utc() {
        assert_eq!(dated("2026-01-01"), dated("2026-01-01T00:00:00Z"));
        assert!(dated("2026-01-01") < dated("2026-01-01T00:00:01Z"));
        assert!(dated("2025-12-31T23:59:59Z") < dated("2026-01-01"));
    }

    #[test]
    fn an_offset_timestamp_is_normalised_to_utc() {
        assert_eq!(
            dated("2026-01-01T02:00:00+02:00"),
            dated("2026-01-01T00:00:00Z")
        );
        assert_eq!(
            dated("2025-12-31T22:41:51-01:30"),
            dated("2026-01-01T00:11:51+00:00")
        );
    }

    #[test]
    fn a_missing_or_unparseable_date_orders_after_every_dated_item() {
        let latest = dated("9999-12-31T23:59:59Z");
        assert!(latest < undated());
        assert!(latest < dated("last tuesday"));
        assert!(latest < dated("2026-13-01"));
        assert_eq!(undated(), dated("not a date"));
    }
}
