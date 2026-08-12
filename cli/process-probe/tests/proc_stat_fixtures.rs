//! The `/proc/<pid>/stat` arithmetic, pinned against the fixtures that
//! recorded the shell launcher's own contract.
//!
//! The parse is a pure function so it runs on every host, not only Linux: the
//! platform read is what needs `/proc`, not the arithmetic on what it returns.

use process_probe::start_time_from_proc_stat;

type TestError = Box<dyn std::error::Error>;

/// The fixtures are labelled key-value files, one field per line, as the
/// retired JavaScript suite read them.
fn field<'a>(fixture: &'a str, key: &str) -> Option<&'a str> {
    fixture
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
}

fn replay(fixture: &str) -> Result<(Option<u64>, u64), TestError> {
    let stat = field(fixture, "stat").ok_or("no stat field")?;
    let btime: u64 = field(fixture, "btime").ok_or("no btime")?.parse()?;
    let hz: u64 = field(fixture, "hz").ok_or("no hz")?.parse()?;
    let expected: u64 = field(fixture, "expected_start_time")
        .ok_or("no expected")?
        .parse()?;
    Ok((start_time_from_proc_stat(stat, btime, hz), expected))
}

#[test]
fn the_recorded_fixture_yields_its_recorded_start_time() -> Result<(), TestError>
{
    let (actual, expected) =
        replay(include_str!("fixtures/proc-stat-linux.txt"))?;
    assert_eq!(actual, Some(expected));
    Ok(())
}

/// Truncating integer division, matching the shell's `$(( ))` and the retired
/// JavaScript's `Math.floor`. Computing `(btime * hz + ticks) / hz` in floating
/// point would differ by up to a second here, which is the entire ±1s tolerance
/// the identity check exists to provide for whole-second-boundary drift.
///
/// The same fixture also carries parentheses and spaces inside its `comm`
/// field, so a non-greedy scan for the closing paren would mis-index every
/// field after it.
#[test]
fn a_tick_count_that_does_not_divide_evenly_truncates() -> Result<(), TestError>
{
    let (actual, expected) =
        replay(include_str!("fixtures/proc-stat-linux-uneven-ticks.txt"))?;
    assert_eq!(actual, Some(expected));
    Ok(())
}

#[test]
fn a_zero_tick_rate_is_unobtainable_rather_than_a_division_by_zero() {
    let stat = include_str!("fixtures/proc-stat-linux.txt");
    let stat = field(stat, "stat").unwrap_or_default();
    assert_eq!(start_time_from_proc_stat(stat, 1_700_000_000, 0), None);
}

#[test]
fn a_malformed_stat_line_is_unobtainable() {
    for stat in ["", "no parens here", "12345 (node)", "()"] {
        assert_eq!(
            start_time_from_proc_stat(stat, 1_700_000_000, 100),
            None,
            "{stat:?}"
        );
    }
}
