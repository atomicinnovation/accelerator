//! A calendar date at the precision its source declared: a year, a month or
//! a day.

use std::fmt;
use std::ops::RangeInclusive;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialDate {
    year: u16,
    month: Option<u8>,
    day: Option<u8>,
}

impl PartialDate {
    /// Reads `YYYY`, `YYYY-MM` or `YYYY-MM-DD`, the last optionally followed
    /// by `T` and a time of day that is checked and then dropped;
    /// `YYYY/M[M]` and `YYYY/M[M]/D[D]`; or an English month name, in full
    /// or abbreviated, as `Month D[D][,] YYYY`, `D[D] Month YYYY` or
    /// `Month YYYY`.
    ///
    /// A time's offset is dropped with it, so the date is the one local to
    /// the declaration. An all-numeric date that is not year-first is
    /// refused, as its day and month order cannot be known.
    pub fn parse(raw: &str) -> Option<Self> {
        if raw.contains('/') {
            Self::slashed(raw)
        } else if raw.contains(char::is_whitespace) {
            Self::month_named(raw)
        } else {
            Self::iso(raw)
        }
    }

    /// The UTC calendar date of `at`; an instant before the epoch counts as
    /// the epoch.
    pub fn utc_date_of(at: SystemTime) -> Self {
        let seconds = at
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        civil_from_days(seconds / SECONDS_PER_DAY)
    }

    fn iso(raw: &str) -> Option<Self> {
        let (date, time_of_day) = match raw.split_once('T') {
            Some((date, time_of_day)) => (date, Some(time_of_day)),
            None => (raw, None),
        };
        let fields = date.split('-').collect::<Vec<_>>();
        let date = Self::from_fields(&fields, 2..=2)?;
        let time_of_day_allowed = time_of_day.is_none_or(|time_of_day| {
            date.day.is_some() && is_time_of_day(time_of_day)
        });
        time_of_day_allowed.then_some(date)
    }

    fn slashed(raw: &str) -> Option<Self> {
        let fields = raw.split('/').collect::<Vec<_>>();
        Self::from_fields(&fields, 1..=2).filter(|date| date.month.is_some())
    }

    fn month_named(raw: &str) -> Option<Self> {
        let words = raw.split_whitespace().collect::<Vec<_>>();
        let (month, day, year) = match words.as_slice() {
            [month, year] => (*month, None, *year),
            [month, day, year] if month_number(month).is_some() => {
                (*month, Some(day.strip_suffix(',').unwrap_or(day)), *year)
            }
            [day, month, year] => (*month, Some(*day), *year),
            _ => return None,
        };
        let day = match day {
            Some(day) => Some(number(day, 1..=2)?),
            None => None,
        };
        Self::validated(number(year, 4..=4)?, Some(month_number(month)?), day)
    }

    fn from_fields(
        fields: &[&str],
        part_widths: RangeInclusive<usize>,
    ) -> Option<Self> {
        let (year, parts) = fields.split_first()?;
        let year = number(year, 4..=4)?;
        let parts = parts
            .iter()
            .map(|part| number(part, part_widths.clone()))
            .collect::<Option<Vec<_>>>()?;
        let (month, day) = match parts.as_slice() {
            [] => (None, None),
            [month] => (Some(*month), None),
            [month, day] => (Some(*month), Some(*day)),
            _ => return None,
        };
        Self::validated(year, month, day)
    }

    fn validated(
        year: u16,
        month: Option<u16>,
        day: Option<u16>,
    ) -> Option<Self> {
        let month = match month {
            None => None,
            Some(month) => Some(
                u8::try_from(month)
                    .ok()
                    .filter(|month| (1..=12).contains(month))?,
            ),
        };
        let day = match (month, day) {
            (_, None) => None,
            (None, Some(_)) => return None,
            (Some(month), Some(day)) => {
                Some(u8::try_from(day).ok().filter(|day| {
                    (1..=days_in_month(year, month)).contains(day)
                })?)
            }
        };
        Some(Self { year, month, day })
    }
}

impl fmt::Display for PartialDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:04}", self.year)?;
        if let Some(month) = self.month {
            write!(formatter, "-{month:02}")?;
        }
        if let Some(day) = self.day {
            write!(formatter, "-{day:02}")?;
        }
        Ok(())
    }
}

const SECONDS_PER_DAY: u64 = 86_400;

const MONTH_NAMES: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// A full name, or its first three letters with an optional `.`; `Sept`
/// is the one longer abbreviation in common use.
fn month_number(word: &str) -> Option<u16> {
    let word = word.to_ascii_lowercase();
    let abbreviation = word.strip_suffix('.').unwrap_or(&word);
    MONTH_NAMES.iter().zip(1..).find_map(|(name, number)| {
        let names_it = word == *name
            || abbreviation == &name[..3]
            || (abbreviation == "sept" && *name == "september");
        names_it.then_some(number)
    })
}

fn number(field: &str, widths: RangeInclusive<usize>) -> Option<u16> {
    (widths.contains(&field.len()) && field.bytes().all(|b| b.is_ascii_digit()))
        .then(|| field.parse().ok())
        .flatten()
}

fn bounded(field: &str, most: u16) -> bool {
    number(field, 2..=2).is_some_and(|value| value <= most)
}

const fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `hh:mm[:ss[.fraction]]` then an optional `Z`, `±hh`, `±hhmm` or `±hh:mm`.
fn is_time_of_day(text: &str) -> bool {
    let (clock, offset) = text
        .find(['Z', '+', '-'])
        .map_or((text, None), |at| (&text[..at], Some(&text[at..])));
    is_clock(clock) && offset.is_none_or(is_offset)
}

fn is_clock(clock: &str) -> bool {
    let mut fields = clock.split(':');
    let hours_and_minutes =
        fields.next().is_some_and(|hours| bounded(hours, 23))
            && fields.next().is_some_and(|minutes| bounded(minutes, 59));
    let seconds = fields.next().is_none_or(|seconds| {
        let (whole, fraction) = seconds
            .split_once('.')
            .map_or((seconds, None), |(whole, fraction)| {
                (whole, Some(fraction))
            });
        bounded(whole, 59)
            && fraction.is_none_or(|fraction| {
                !fraction.is_empty()
                    && fraction.bytes().all(|b| b.is_ascii_digit())
            })
    });
    hours_and_minutes && seconds && fields.next().is_none()
}

fn is_offset(offset: &str) -> bool {
    if offset == "Z" {
        return true;
    }
    let Some(magnitude) = offset.strip_prefix(['+', '-']) else {
        return false;
    };
    let (hours, minutes) = match magnitude.len() {
        2 => (magnitude, None),
        4 => (&magnitude[..2], Some(&magnitude[2..])),
        5 => match magnitude.split_once(':') {
            Some((hours, minutes)) => (hours, Some(minutes)),
            None => return false,
        },
        _ => return false,
    };
    bounded(hours, 23) && minutes.is_none_or(|minutes| bounded(minutes, 59))
}

/// Howard Hinnant's `civil_from_days`, over days since 1970-01-01.
fn civil_from_days(days: u64) -> PartialDate {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    PartialDate {
        year: u16::try_from(year).unwrap_or(u16::MAX),
        month: u8::try_from(month).ok(),
        day: u8::try_from(day).ok(),
    }
}
