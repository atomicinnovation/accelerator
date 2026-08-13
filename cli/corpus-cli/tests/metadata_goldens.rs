//! `accelerator-corpus metadata derive` against the block shape its consumers
//! hold it to, replayed inside a hermetically isolated git and jj tempdir.
//!
//! Covers the success path only — the one concrete Rust-side failure
//! condition (`SystemClock::try_new`'s `ClockError`, driven by host tzdata
//! availability) can't be forced hermetically through the compiled binary,
//! so it is exercised at the unit level in `src/metadata.rs` instead.

use std::fs;
use std::path::Path;
use std::process::Command;

use vcs_test_support::hermetic::assert_no_repository_ancestor;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-corpus");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("corpus-metadata-golden-{tag}-"))
        .tempdir()?)
}

fn derive_in(dir: &Path, env: &Hermetic) -> Result<String, TestError> {
    derive_in_with(dir, env, &[])
}

fn derive_in_with(
    dir: &Path,
    env: &Hermetic,
    arguments: &[&str],
) -> Result<String, TestError> {
    let mut command = Command::new(BIN);
    command.current_dir(dir);
    command.args(["metadata", "derive"]);
    command.args(arguments);
    env.apply(&mut command);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "accelerator-corpus metadata derive failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// The block shape `corpus-adapters` pins for its own `derive`/`render`,
/// duplicated here because a separate test binary cannot import another
/// crate's private test helper.
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

    assert!(
        block.contains("Timestamp For Filename: "),
        "both formats this command exposes carry a time of day, so both are \
         labelled a timestamp:\n{block}"
    );
}

#[test]
fn a_git_repository_satisfies_the_helper_contract() -> Result<(), TestError> {
    let work = tempdir("git")?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &repo)?;

    let block = derive_in(&repo, &env)?;
    assert_satisfies_the_helper_contract(&block);
    assert!(block.contains("Repository Name: repo"));
    Ok(())
}

#[test]
fn a_jj_repository_satisfies_the_helper_contract() -> Result<(), TestError> {
    let work = tempdir("jj")?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.jj(&["git", "init", "--colocate"], &repo)?;

    let block = derive_in(&repo, &env)?;
    assert_satisfies_the_helper_contract(&block);
    assert!(block.contains("Repository Name: repo"));
    Ok(())
}

/// The `%Y-%m-%d-%H%M%S` shape. The renderer itself is pinned digit-for-digit
/// against a fixed instant in `corpus-adapters`; `derive_at` builds its own
/// `SystemClock`, so through the compiled binary only the shape is observable.
fn assert_is_a_compact_time_stamp(stamp: &str) {
    let fields: Vec<&str> = stamp.split('-').collect();
    assert_eq!(
        fields
            .as_slice()
            .iter()
            .map(|field| field.len())
            .collect::<Vec<_>>(),
        vec![4, 2, 2, 6],
        "the compact-time stamp must be YYYY-MM-DD-HHMMSS, got {stamp:?}"
    );
    assert!(
        fields
            .iter()
            .all(|field| field.bytes().all(|b| b.is_ascii_digit())),
        "every component must be digits, got {stamp:?}"
    );
}

#[test]
fn the_compact_time_format_renders_a_date_and_a_time_of_day(
) -> Result<(), TestError> {
    let work = tempdir("compact")?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &repo)?;

    let block = derive_in_with(
        &repo,
        &env,
        &["--filename-timestamp-format", "compact-time"],
    )?;
    assert_satisfies_the_helper_contract(&block);

    let stamp = block
        .lines()
        .find_map(|line| line.strip_prefix("Timestamp For Filename: "))
        .ok_or("no Timestamp For Filename line")?;
    assert_is_a_compact_time_stamp(stamp);
    Ok(())
}

#[test]
fn omitting_the_format_keeps_today_s_underscored_stamp() -> Result<(), TestError>
{
    let work = tempdir("default")?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &repo)?;

    let block = derive_in(&repo, &env)?;
    let stamp = block
        .lines()
        .find_map(|line| line.strip_prefix("Timestamp For Filename: "))
        .ok_or("no Timestamp For Filename line")?;
    assert_eq!(stamp.len(), "2026-07-13_09-05-07".len());
    assert!(
        stamp.contains('_'),
        "the default stamp separates date from time with an underscore, got \
         {stamp:?}"
    );
    Ok(())
}

/// The four lines the design skills consume, in order, with their labels.
///
/// The timestamp component is pinned byte-for-byte against a fixed instant in
/// `corpus-adapters`; here only labels and order are asserted, because
/// `derive_at` builds its own clock and the revision is the repository's.
fn labels_of(block: &str) -> Vec<&str> {
    block
        .lines()
        .filter_map(|line| line.split_once(": ").map(|(label, _)| label))
        .collect()
}

/// Both filename-stamp formats the design skills ask for.
#[test]
fn each_filename_stamp_format_renders_its_own_block() -> Result<(), TestError> {
    for (format, stamp_label) in [
        ("compact-time", "Timestamp For Filename"),
        ("date-only", "Date For Filename"),
    ] {
        let work = tempdir(format)?;
        let env = Hermetic::rooted_at(work.path())?;
        let repo = work.path().join("repo");
        fs::create_dir_all(&repo)?;
        env.git(&["init", "--quiet"], &repo)?;
        env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &repo)?;

        let block = derive_in_with(
            &repo,
            &env,
            &["--filename-timestamp-format", format],
        )?;
        assert_eq!(
            labels_of(&block),
            vec![
                "Current Date/Time (UTC)",
                stamp_label,
                "Current Revision",
                "Repository Name",
            ],
            "{format} drifted from the expected labels or ordering:\n{block}"
        );
    }
    Ok(())
}

/// The `Date For Filename` label carries `%Y-%m-%d` — a different format from
/// the compact stamp, and easy to conflate with it.
#[test]
fn the_date_only_format_renders_a_date_without_a_time() -> Result<(), TestError>
{
    let work = tempdir("date-only-shape")?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("repo");
    fs::create_dir_all(&repo)?;
    env.git(&["init", "--quiet"], &repo)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &repo)?;

    let block = derive_in_with(
        &repo,
        &env,
        &["--filename-timestamp-format", "date-only"],
    )?;
    let stamp = block
        .lines()
        .find_map(|line| line.strip_prefix("Date For Filename: "))
        .ok_or("no Date For Filename line")?;
    let fields: Vec<&str> = stamp.split('-').collect();
    assert_eq!(
        fields.iter().map(|field| field.len()).collect::<Vec<_>>(),
        vec![4, 2, 2],
        "the date-only stamp must be YYYY-MM-DD, got {stamp:?}"
    );
    assert!(
        !block.contains("Timestamp For Filename"),
        "a format carrying no time of day is labelled a date:\n{block}"
    );
    Ok(())
}

#[test]
fn outside_a_repository_the_provenance_lines_are_omitted(
) -> Result<(), TestError> {
    let work = tempdir("bare")?;
    // `gix::discover` reads no environment, so `GIT_CEILING_DIRECTORIES`
    // cannot fence its in-process walk the way it fences a real `git`
    // subprocess — assert no repository ancestor explicitly rather than
    // risk silently resolving an enclosing checkout on a host whose tmpdir
    // sits inside one.
    assert_no_repository_ancestor(work.path())?;
    let env = Hermetic::rooted_at(work.path())?;
    let plain = work.path().join("plain");
    fs::create_dir_all(&plain)?;

    let block = derive_in(&plain, &env)?;
    assert!(block.contains("Current Date/Time (UTC): "));
    assert!(!block.contains("Current Revision:"));
    assert!(!block.contains("Repository Name:"));
    Ok(())
}
