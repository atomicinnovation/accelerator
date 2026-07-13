//! Artifact-metadata derivation: the clock adapter, and the composition of a
//! clock and a repository probe into the block the authoring skills stamp
//! artifacts with.
//!
//! This subsumes the three bash metadata helpers, which differ only in the
//! filename timestamp they render.

use std::fmt;
use std::path::Path;
use std::process::Command;

use corpus::{ArtifactMetadata, Clock, FilenameTimestampFormat};
use time::{OffsetDateTime, UtcOffset};
use vcs::RepoFacts;

/// The host's UTC offset could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockError(String);

impl fmt::Display for ClockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "could not resolve the host's UTC offset: {}",
            self.0
        )
    }
}

impl std::error::Error for ClockError {}

/// Renders `instant` as the UTC ISO line, with a literal `+00:00` rather than a
/// zone abbreviation.
#[must_use]
pub fn format_utc_iso(instant: OffsetDateTime) -> String {
    let instant = instant.to_offset(UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        instant.year(),
        u8::from(instant.month()),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second()
    )
}

/// Renders `instant` in the shape the given helper's filenames use. `instant`
/// is rendered in whatever offset it already carries, so a caller wanting a
/// host-local stamp passes a host-local instant.
#[must_use]
pub fn format_filename_timestamp(
    instant: OffsetDateTime,
    format: FilenameTimestampFormat,
) -> String {
    let year = instant.year();
    let month = u8::from(instant.month());
    let day = instant.day();
    let (hour, minute, second) =
        (instant.hour(), instant.minute(), instant.second());

    match format {
        FilenameTimestampFormat::DateOnly => {
            format!("{year:04}-{month:02}-{day:02}")
        }
        FilenameTimestampFormat::DateTimeUnderscored => format!(
            "{year:04}-{month:02}-{day:02}_{hour:02}-{minute:02}-{second:02}"
        ),
        FilenameTimestampFormat::CompactTime => format!(
            "{year:04}-{month:02}-{day:02}-{hour:02}{minute:02}{second:02}"
        ),
    }
}

/// The label the helpers give the filename line. The date-only helper calls it
/// a date; the two that carry a time call it a timestamp.
const fn filename_label(format: FilenameTimestampFormat) -> &'static str {
    match format {
        FilenameTimestampFormat::DateOnly => "Date For Filename",
        FilenameTimestampFormat::DateTimeUnderscored
        | FilenameTimestampFormat::CompactTime => "Timestamp For Filename",
    }
}

/// The real clock: UTC for the ISO line, host-local for the filename stamp,
/// matching bash's `date -u` and plain `date` respectively.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    offset: UtcOffset,
}

impl SystemClock {
    /// Resolves the host's UTC offset once and caches it.
    ///
    /// The offset is the clock's only I/O. `time` refuses to resolve a local
    /// offset in a multithreaded process — which is every process this runs in
    /// — so it is read from a short-lived subprocess instead, which is
    /// single-threaded by construction.
    ///
    /// # Errors
    ///
    /// Returns [`ClockError`] when the offset cannot be resolved, rather than
    /// falling back to UTC and stamping artifacts with a wrong-zone
    /// provenance. This is a deliberate divergence from the bash helpers, which
    /// degrade silently; `tzdata` or `TZ` is a runtime prerequisite.
    pub fn try_new() -> Result<Self, ClockError> {
        let output = Command::new("date")
            .arg("+%z")
            .output()
            .map_err(|error| ClockError(error.to_string()))?;
        if !output.status.success() {
            return Err(ClockError(format!(
                "`date +%z` exited {}",
                output.status
            )));
        }

        let raw = String::from_utf8(output.stdout)
            .map_err(|error| ClockError(error.to_string()))?;
        let offset = parse_offset(raw.trim()).ok_or_else(|| {
            ClockError(format!("`date +%z` said {:?}", raw.trim()))
        })?;

        Ok(Self { offset })
    }

    /// A clock pinned to `offset`, so a caller — or a test — can render in a
    /// zone other than the host's.
    #[must_use]
    pub const fn with_offset(offset: UtcOffset) -> Self {
        Self { offset }
    }

    #[must_use]
    pub const fn offset(self) -> UtcOffset {
        self.offset
    }
}

/// Parses the `±HHMM` an RFC-822 style offset is printed as.
fn parse_offset(raw: &str) -> Option<UtcOffset> {
    let sign = match raw.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits = raw.get(1..)?;
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let hours: i8 = digits.get(0..2)?.parse().ok()?;
    let minutes: i8 = digits.get(2..4)?.parse().ok()?;
    UtcOffset::from_hms(sign * hours, sign * minutes, 0).ok()
}

impl Clock for SystemClock {
    fn now_utc_iso(&self) -> String {
        format_utc_iso(OffsetDateTime::now_utc())
    }

    fn filename_timestamp(&self, format: FilenameTimestampFormat) -> String {
        format_filename_timestamp(
            OffsetDateTime::now_utc().to_offset(self.offset),
            format,
        )
    }
}

/// Composes the metadata from a clock and the repository's facts.
///
/// `facts` is `None` outside a repository, which blanks the repository name and
/// revision rather than fabricating them. A *failed* probe blanks the revision
/// the same way, so a consumer that persists this block must surface the
/// probe's warning rather than silently writing blank provenance.
#[must_use]
pub fn derive(
    clock: &dyn Clock,
    facts: Option<&RepoFacts>,
    format: FilenameTimestampFormat,
) -> ArtifactMetadata {
    ArtifactMetadata {
        datetime_utc: clock.now_utc_iso(),
        filename_timestamp: clock.filename_timestamp(format),
        repository_name: facts.map(|facts| facts.name.clone()),
        revision: facts.and_then(|facts| facts.revision.clone()),
    }
}

/// Derives the metadata for an artifact authored under `start`, against the
/// real clock and the real repository.
///
/// # Errors
///
/// Returns [`ClockError`] when the host's UTC offset cannot be resolved.
pub fn derive_at(
    start: &Path,
    format: FilenameTimestampFormat,
) -> Result<ArtifactMetadata, ClockError> {
    let clock = SystemClock::try_new()?;
    let facts = vcs_adapters::facts(start);
    Ok(derive(&clock, facts.as_ref(), format))
}

/// Renders the metadata as the labelled block the bash helpers print and the
/// authoring skills read. Absent facts drop their line entirely, as in bash.
#[must_use]
pub fn render(
    metadata: &ArtifactMetadata,
    format: FilenameTimestampFormat,
) -> String {
    use std::fmt::Write as _;

    let mut block = format!(
        "Current Date/Time (UTC): {}\n{}: {}\n",
        metadata.datetime_utc,
        filename_label(format),
        metadata.filename_timestamp
    );
    if let Some(revision) = &metadata.revision {
        let _ = writeln!(block, "Current Revision: {revision}");
    }
    if let Some(name) = &metadata.repository_name {
        let _ = writeln!(block, "Repository Name: {name}");
    }
    block
}

#[cfg(test)]
mod tests {
    use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

    use super::{
        format_filename_timestamp, format_utc_iso, parse_offset,
        FilenameTimestampFormat,
    };

    type TestError = Box<dyn std::error::Error>;

    /// 2026-07-13T09:05:07Z — deliberately single-digit month, day, hour,
    /// minute, and second, so a missing zero-pad cannot pass.
    fn instant() -> Result<OffsetDateTime, TestError> {
        let date = Date::from_calendar_date(2026, Month::July, 13)?;
        let time = Time::from_hms(9, 5, 7)?;
        Ok(date.with_time(time).assume_utc())
    }

    #[test]
    fn the_utc_line_is_iso_with_a_literal_offset() -> Result<(), TestError> {
        assert_eq!(format_utc_iso(instant()?), "2026-07-13T09:05:07+00:00");
        Ok(())
    }

    #[test]
    fn the_utc_line_is_rendered_in_utc_whatever_offset_it_carries(
    ) -> Result<(), TestError> {
        let local = instant()?.to_offset(UtcOffset::from_hms(5, 30, 0)?);
        assert_eq!(format_utc_iso(local), "2026-07-13T09:05:07+00:00");
        Ok(())
    }

    #[test]
    fn each_helper_s_filename_format_is_pinned() -> Result<(), TestError> {
        let instant = instant()?;
        assert_eq!(
            format_filename_timestamp(
                instant,
                FilenameTimestampFormat::DateTimeUnderscored
            ),
            "2026-07-13_09-05-07"
        );
        assert_eq!(
            format_filename_timestamp(
                instant,
                FilenameTimestampFormat::CompactTime
            ),
            "2026-07-13-090507"
        );
        assert_eq!(
            format_filename_timestamp(
                instant,
                FilenameTimestampFormat::DateOnly
            ),
            "2026-07-13"
        );
        Ok(())
    }

    #[test]
    fn the_filename_stamp_follows_the_instant_s_offset() -> Result<(), TestError>
    {
        let local = instant()?.to_offset(UtcOffset::from_hms(5, 30, 0)?);
        assert_eq!(
            format_filename_timestamp(
                local,
                FilenameTimestampFormat::DateTimeUnderscored
            ),
            "2026-07-13_14-35-07",
            "a host-local instant must render in its own zone, not UTC"
        );
        Ok(())
    }

    #[test]
    fn offsets_parse_in_both_directions() {
        assert_eq!(parse_offset("+0000"), Some(UtcOffset::UTC));
        assert_eq!(parse_offset("+0530"), UtcOffset::from_hms(5, 30, 0).ok());
        assert_eq!(parse_offset("-0800"), UtcOffset::from_hms(-8, 0, 0).ok());
    }

    #[test]
    fn a_malformed_offset_is_rejected() {
        for raw in ["", "+", "0530", "+05:30", "+05300", "+abcd"] {
            assert_eq!(parse_offset(raw), None, "should not parse: {raw:?}");
        }
    }
}
