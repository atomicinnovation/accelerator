//! Black-box tests of the compiled `accelerator config` read surface.
//!
//! Each test builds a throwaway workspace under `CARGO_TARGET_TMPDIR` carrying a
//! `.git` boundary marker, so root discovery is bounded inside the fixture
//! rather than escaping into the real working tree. Byte-exact assertions
//! compare `output.stdout` directly, never through `from_utf8_lossy`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

type TestResult = Result<(), Box<dyn Error>>;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A throwaway workspace with a `.git` boundary marker.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "config-read-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".git"))?;
        Ok(Self { root })
    }

    fn team(self, body: &str) -> Result<Self, Box<dyn Error>> {
        fs::create_dir_all(self.root.join(".accelerator"))?;
        fs::write(self.root.join(".accelerator/config.md"), body)?;
        Ok(self)
    }

    fn legacy(self, body: &str) -> Result<Self, Box<dyn Error>> {
        fs::create_dir_all(self.root.join(".claude"))?;
        fs::write(self.root.join(".claude/accelerator.md"), body)?;
        Ok(self)
    }

    fn run(&self, args: &[&str]) -> Result<Output, Box<dyn Error>> {
        run_in(&self.root, args)
    }
}

fn run_in(cwd: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator"));
    command.current_dir(cwd);
    command.env_remove("ACCELERATOR_LOG");
    command.env_remove("CLAUDE_PLUGIN_ROOT");
    command.env_remove("ACCELERATOR_CACHE_DIR");
    command.env_remove("ACCELERATOR_RELEASE_BASE_URL");
    command.args(args);
    Ok(command.output()?)
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

const SEEDED: &str = "\
---
agents:
  reviewer: my-reviewer
paths:
  work: custom/work
work:
  integration: linear
---
body prose
";

#[test]
fn get_prints_a_set_value_and_exits_zero() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "get", "agents.reviewer"])?;
    assert_eq!(output.stdout, b"my-reviewer\n");
    assert!(output.stderr.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn get_of_an_unset_key_without_a_default_prints_empty_and_exits_zero(
) -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "get", "missing.key"])?;
    assert_eq!(output.stdout, b"\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn get_of_an_unset_key_prints_the_callers_default() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "get", "missing.key", "fallback"])?;
    assert_eq!(output.stdout, b"fallback\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

/// The presence probe `jira-auth.sh:228` / `write-visualiser-config.sh:64-65`
/// depend on: an explicitly empty default yields empty on a miss, with no
/// catalogue lookup for `get`.
#[test]
fn get_with_an_explicit_empty_default_yields_empty_on_a_miss() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "get", "review.min_lenses", ""])?;
    assert_eq!(output.stdout, b"\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn path_prints_a_set_value() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "path", "work"])?;
    assert_eq!(output.stdout, b"custom/work\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn path_of_an_unset_key_falls_back_to_the_catalogue_default() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "path", "plans"])?;
    assert_eq!(output.stdout, b"meta/plans\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn path_prefers_an_explicit_default_over_the_catalogue() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output =
        fixture.run(&["config", "path", "plans", "elsewhere/plans"])?;
    assert_eq!(output.stdout, b"elsewhere/plans\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn path_of_an_unknown_key_warns_on_stderr_and_prints_empty() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "path", "bogus"])?;
    assert_eq!(output.stdout, b"\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown key"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn agent_prints_a_set_override() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "agent", "reviewer"])?;
    assert_eq!(output.stdout, b"my-reviewer\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn agent_of_an_unset_key_falls_back_to_the_prefixed_default() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "agent", "codebase-locator"])?;
    assert_eq!(output.stdout, b"accelerator:codebase-locator\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

const MALFORMED: &str = "---\nkey: value\n";

#[test]
fn a_scalar_with_fail_safe_suppresses_a_read_failure_and_exits_zero(
) -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output =
        fixture.run(&["config", "get", "agents.reviewer", "--fail-safe"])?;
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn a_scalar_without_fail_safe_fails_loud_on_a_read_failure() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "get", "agents.reviewer"])?;
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn a_legacy_layout_is_refused_by_a_read() -> TestResult {
    let fixture =
        Fixture::new()?.legacy("---\npaths:\n  work: legacy\n---\n")?;
    let output = fixture.run(&["config", "path", "work"])?;
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("legacy config"));
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn allow_legacy_layout_suppresses_the_refusal_and_reads_the_legacy_pair(
) -> TestResult {
    let fixture =
        Fixture::new()?.legacy("---\npaths:\n  work: legacy\n---\n")?;
    let output =
        fixture.run(&["config", "path", "work", "--allow-legacy-layout"])?;
    assert_eq!(output.stdout, b"legacy\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn the_legacy_fallback_is_inert_when_the_current_pair_is_present() -> TestResult
{
    let fixture = Fixture::new()?
        .team("---\npaths:\n  work: current\n---\n")?
        .legacy("---\npaths:\n  work: legacy\n---\n")?;
    let output =
        fixture.run(&["config", "path", "work", "--allow-legacy-layout"])?;
    assert_eq!(output.stdout, b"current\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn a_usage_error_exits_one_not_two() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let bad_level = fixture.run(&["config", "get", "x", "--level", "bogus"])?;
    assert_eq!(code(&bad_level), 1);
    let unknown_flag = fixture.run(&["config", "get", "x", "--nope"])?;
    assert_eq!(code(&unknown_flag), 1);
    let top_level = fixture.run(&["--bogus-flag"])?;
    assert_eq!(code(&top_level), 1);
    Ok(())
}

#[test]
fn a_bare_config_prints_help_and_exits_zero() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config"])?;
    assert_eq!(code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage"));
    Ok(())
}

#[test]
fn subcommand_help_renders_the_matched_subcommand_not_the_root() -> TestResult {
    let fixture = Fixture::new()?.team(SEEDED)?;
    let output = fixture.run(&["config", "get", "--help"])?;
    assert_eq!(code(&output), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Print a configuration value"));
    Ok(())
}
