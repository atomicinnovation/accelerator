//! Artifact-metadata derivation, asserted behind faked ports so every field is
//! deterministic, plus the block contract its consumers are held to.

mod common;

use common::TestError;
use corpus::{
    ArtifactMetadata, Clock, FilenameTimestampFormat, RepositoryFacts,
};
use corpus_adapters::metadata::{derive, render, SystemClock};
use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

const FORMATS: [FilenameTimestampFormat; 3] = [
    FilenameTimestampFormat::DateTimeUnderscored,
    FilenameTimestampFormat::DateOnly,
    FilenameTimestampFormat::CompactTime,
];

struct FakeClock;

impl Clock for FakeClock {
    fn now_utc_iso(&self) -> String {
        "2026-07-13T09:05:07+00:00".to_owned()
    }

    fn filename_timestamp(&self, format: FilenameTimestampFormat) -> String {
        match format {
            FilenameTimestampFormat::DateTimeUnderscored => {
                "2026-07-13_09-05-07".to_owned()
            }
            FilenameTimestampFormat::DateOnly => "2026-07-13".to_owned(),
            FilenameTimestampFormat::CompactTime => {
                "2026-07-13-090507".to_owned()
            }
        }
    }
}

fn facts(revision: Option<&str>) -> RepositoryFacts {
    RepositoryFacts {
        name: "accelerator".to_owned(),
        revision: revision.map(str::to_owned),
    }
}

#[test]
fn every_field_is_derived_from_the_clock_and_the_repository() {
    let facts = facts(Some("84a6b82ae0ee9964a036d54cd5e7a00db01bad11"));

    assert_eq!(
        derive(
            &FakeClock,
            Some(&facts),
            FilenameTimestampFormat::DateTimeUnderscored
        ),
        ArtifactMetadata {
            datetime_utc: "2026-07-13T09:05:07+00:00".to_owned(),
            filename_timestamp: "2026-07-13_09-05-07".to_owned(),
            repository_name: Some("accelerator".to_owned()),
            revision: Some(
                "84a6b82ae0ee9964a036d54cd5e7a00db01bad11".to_owned()
            ),
        }
    );
}

#[test]
fn no_repository_blanks_the_name_and_the_revision() {
    let derived = derive(
        &FakeClock,
        None,
        FilenameTimestampFormat::DateTimeUnderscored,
    );

    assert_eq!(derived.repository_name, None);
    assert_eq!(derived.revision, None);
    assert_eq!(
        derived.datetime_utc, "2026-07-13T09:05:07+00:00",
        "the timestamps stand alone — they do not need a repository"
    );
}

#[test]
fn an_unanswerable_revision_blanks_only_the_revision() {
    let derived = derive(
        &FakeClock,
        Some(&facts(None)),
        FilenameTimestampFormat::DateOnly,
    );

    assert_eq!(derived.revision, None);
    assert_eq!(
        derived.repository_name.as_deref(),
        Some("accelerator"),
        "a failed probe must not take the repository name down with it"
    );
}

/// The shape every derived metadata block holds to.
fn assert_satisfies_the_helper_contract(block: &str) {
    let lines: Vec<&str> = block.lines().collect();

    let revision = lines
        .iter()
        .find_map(|line| line.strip_prefix("Current Revision: "));
    assert!(
        revision.is_some_and(|value| !value.trim().is_empty()),
        "the Current Revision label must be present and non-empty:\n{block}"
    );

    let datetime = lines
        .iter()
        .find_map(|line| line.strip_prefix("Current Date/Time (UTC): "))
        .unwrap_or_default();
    assert!(
        datetime.ends_with("+00:00")
            && datetime.len() == "2026-07-13T09:05:07+00:00".len(),
        "the datetime must be ISO with a literal +00:00, got {datetime:?}"
    );

    for forbidden in [
        "Current Branch Name:",
        "GIT_BRANCH=",
        "Current Git Commit Hash:",
    ] {
        assert!(
            !block.contains(forbidden),
            "the retired label {forbidden:?} must not reappear:\n{block}"
        );
    }

    assert!(
        !block.contains(" UTC") && !block.contains(" GMT"),
        "a %Z-style zone abbreviation must not appear:\n{block}"
    );
}

#[test]
fn every_filename_format_renders_a_block_satisfying_the_helper_contract() {
    let facts = facts(Some("84a6b82ae0ee9964a036d54cd5e7a00db01bad11"));

    for format in FORMATS {
        let block = render(&derive(&FakeClock, Some(&facts), format), format);
        assert_satisfies_the_helper_contract(&block);
    }
}

#[test]
fn the_filename_line_is_labelled_the_way_its_helper_labels_it() {
    let facts = facts(Some("abc"));
    let block =
        |format| render(&derive(&FakeClock, Some(&facts), format), format);

    assert!(block(FilenameTimestampFormat::DateTimeUnderscored)
        .contains("Timestamp For Filename: 2026-07-13_09-05-07"));
    assert!(block(FilenameTimestampFormat::CompactTime)
        .contains("Timestamp For Filename: 2026-07-13-090507"));
    assert!(
        block(FilenameTimestampFormat::DateOnly)
            .contains("Date For Filename: 2026-07-13"),
        "the date-only helper labels its line a date, not a timestamp"
    );
}

#[test]
fn a_repository_less_block_omits_the_revision_and_name_lines() {
    let block = render(
        &derive(&FakeClock, None, FilenameTimestampFormat::DateOnly),
        FilenameTimestampFormat::DateOnly,
    );

    assert!(!block.contains("Current Revision:"));
    assert!(!block.contains("Repository Name:"));
    assert!(block.contains("Current Date/Time (UTC):"));
}

#[test]
fn the_host_offset_resolves_rather_than_degrading_to_utc(
) -> Result<(), TestError> {
    // Not an assertion that the host is non-UTC — only that the offset was
    // genuinely resolved, so a tzdata-less host fails loudly here.
    SystemClock::try_new()?;
    Ok(())
}

#[test]
fn a_non_utc_clock_stamps_filenames_locally_and_the_iso_line_in_utc(
) -> Result<(), TestError> {
    // Pinned to a fixed offset rather than the ambient TZ, so the assertion is
    // not vacuous on a UTC-configured CI host.
    let clock = SystemClock::with_offset(UtcOffset::from_hms(5, 30, 0)?);

    let iso = clock.now_utc_iso();
    let stamp =
        clock.filename_timestamp(FilenameTimestampFormat::DateTimeUnderscored);

    assert!(iso.ends_with("+00:00"), "the ISO line must stay UTC: {iso}");

    let utc_hour = &iso[11..13];
    let local_hour = &stamp[11..13];
    assert_ne!(
        utc_hour, local_hour,
        "a +05:30 clock must stamp filenames in local time, not UTC \
         (iso {iso}, stamp {stamp})"
    );
    Ok(())
}

#[test]
fn the_clock_renders_a_known_instant_exactly() -> Result<(), TestError> {
    let instant = Date::from_calendar_date(2026, Month::July, 13)?
        .with_time(Time::from_hms(23, 45, 7)?)
        .assume_utc();

    assert_eq!(
        corpus_adapters::metadata::format_utc_iso(instant),
        "2026-07-13T23:45:07+00:00"
    );
    assert_eq!(
        corpus_adapters::metadata::format_filename_timestamp(
            instant.to_offset(UtcOffset::from_hms(5, 30, 0)?),
            FilenameTimestampFormat::CompactTime
        ),
        "2026-07-14-051507",
        "crossing midnight locally must roll the local date, not the UTC one"
    );
    Ok(())
}

#[test]
fn now_is_not_the_unix_epoch() -> Result<(), TestError> {
    // Guards the clock actually reading the system time rather than a default.
    let clock = SystemClock::try_new()?;
    let now = OffsetDateTime::now_utc();
    assert!(clock.now_utc_iso().starts_with(&now.year().to_string()));
    Ok(())
}
