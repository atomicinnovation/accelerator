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

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Materializes a committed fixture into a fresh temp workspace: its `config.md`
/// (and `config.local.md`, if present) copied under `.accelerator/`, with a
/// `.git` boundary marker so root discovery stops inside the workspace.
fn workspace(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let src = PathBuf::from(FIXTURES).join(name);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "config-read-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join(".accelerator"))?;
    for name in ["config.md", "config.local.md"] {
        let file = src.join(name);
        if file.exists() {
            fs::copy(&file, root.join(".accelerator").join(name))?;
        }
    }
    for subtree in ["skills", "lenses", "tmp"] {
        let source = src.join(subtree);
        if source.is_dir() {
            copy_tree(&source, &root.join(".accelerator").join(subtree))?;
        }
    }
    Ok(root)
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn golden(name: &str, file: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(fs::read(PathBuf::from(FIXTURES).join(name).join(file))?)
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

#[test]
fn agents_matches_the_committed_golden() -> TestResult {
    let workspace = workspace("agents")?;
    let output = run_in(&workspace, &["config", "agents"])?;
    assert_eq!(output.stdout, golden("agents", "agents.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn agents_against_the_baseline_matches_its_golden() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "agents"])?;
    assert_eq!(output.stdout, golden("baseline", "agents.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn agents_warns_on_an_unknown_key_and_still_emits_the_block() -> TestResult {
    let workspace = workspace("agents")?;
    let output = run_in(&workspace, &["config", "agents"])?;
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("unknown agent key 'bogus-agent'"));
    Ok(())
}

#[test]
fn agents_with_fail_safe_renders_the_unavailable_notice() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "agents", "--fail-safe"])?;
    assert_eq!(output.stdout, b"## Agent Names Unavailable\n");
    assert!(!output.stderr.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn work_prints_a_valid_integration() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "work", "integration"])?;
    assert_eq!(output.stdout, b"linear\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn work_prints_a_catalogue_default() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "work", "id_pattern"])?;
    assert_eq!(output.stdout, b"{number:04d}\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn work_with_a_bad_integration_enum_fails_closed() -> TestResult {
    let workspace = workspace("bad-integration")?;
    let output = run_in(&workspace, &["config", "work", "integration"])?;
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("must be one of"));
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn work_stays_fail_closed_on_a_bad_enum_even_with_fail_safe() -> TestResult {
    let workspace = workspace("bad-integration")?;
    let output = run_in(
        &workspace,
        &["config", "work", "integration", "--fail-safe"],
    )?;
    assert!(output.stdout.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn context_matches_the_project_golden() -> TestResult {
    let workspace = workspace("context-full")?;
    let output = run_in(&workspace, &["config", "context"])?;
    assert_eq!(output.stdout, golden("context-full", "context.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_skill_joins_both_blocks_with_one_blank_line() -> TestResult {
    let workspace = workspace("context-full")?;
    let output = run_in(&workspace, &["config", "context", "--skill", "demo"])?;
    assert_eq!(
        output.stdout,
        golden("context-full", "context-skill.golden")?
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn instructions_matches_its_golden() -> TestResult {
    let workspace = workspace("context-full")?;
    let output = run_in(&workspace, &["config", "instructions", "demo"])?;
    assert_eq!(
        output.stdout,
        golden("context-full", "instructions.golden")?
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_of_an_unconfigured_repo_prints_nothing() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output = fixture.run(&["config", "context"])?;
    assert!(output.stdout.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_skill_only_when_the_project_body_is_empty() -> TestResult {
    let workspace = workspace("context-full")?;
    // Blank the project body but keep the skill body.
    fs::write(
        workspace.join(".accelerator/config.md"),
        "---\npaths:\n  work: x\n---\n",
    )?;
    fs::remove_file(workspace.join(".accelerator/config.local.md"))?;
    let output = run_in(&workspace, &["config", "context", "--skill", "demo"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("## Skill-Specific Context\n"));
    assert!(!stdout.contains("## Project Context"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_skill_degrades_the_skill_source_within_one_invocation() -> TestResult
{
    let workspace = workspace("context-full")?;
    // A skill directory whose context.md is a directory, so the read fails.
    fs::remove_file(workspace.join(".accelerator/skills/demo/context.md"))?;
    fs::create_dir(workspace.join(".accelerator/skills/demo/context.md"))?;
    let output = run_in(
        &workspace,
        &["config", "context", "--skill", "demo", "--fail-safe"],
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("## Project Context\n"));
    assert!(stdout.contains("## Skill-Specific Context Unavailable"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn an_invalid_skill_name_under_fail_safe_keeps_the_project_block() -> TestResult
{
    let workspace = workspace("context-full")?;
    let output = run_in(
        &workspace,
        &["config", "context", "--skill", "../../etc", "--fail-safe"],
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("## Project Context\n"));
    assert!(stdout.contains("## Skill-Specific Context Unavailable"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn an_invalid_skill_name_without_fail_safe_exits_non_zero() -> TestResult {
    let workspace = workspace("context-full")?;
    let output =
        run_in(&workspace, &["config", "context", "--skill", "../../etc"])?;
    assert!(output.stdout.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn instructions_of_an_absent_skill_prints_nothing() -> TestResult {
    let workspace = workspace("context-full")?;
    let output = run_in(&workspace, &["config", "instructions", "nonesuch"])?;
    assert!(output.stdout.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_matches_the_configured_golden() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "paths"])?;
    assert_eq!(output.stdout, golden("baseline", "paths.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_all_includes_the_excluded_keys() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "paths", "--all"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- tmp: "));
    assert!(stdout.contains("- templates: "));
    assert!(stdout.contains("- integrations: "));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_doc_types_matches_the_tsv_golden() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(
        &workspace,
        &["config", "paths", "--doc-types", "--format", "tsv"],
    )?;
    assert_eq!(output.stdout, golden("baseline", "doctypes.golden")?);
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 13);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_doc_types_resolves_against_the_root_positional() -> TestResult {
    let workspace = workspace("baseline")?;
    // Run from an unrelated CWD, pointing the resolver at the workspace root.
    let elsewhere = workspace.parent().unwrap_or(&workspace).to_path_buf();
    let output = run_in(
        &elsewhere,
        &[
            "config",
            "paths",
            "--doc-types",
            "--format",
            "tsv",
            workspace.to_str().unwrap_or("."),
        ],
    )?;
    assert_eq!(output.stdout, golden("baseline", "doctypes.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_doc_types_coerces_a_blank_key_to_thirteen_rows() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: \"\"\n---\n")?;
    let output = fixture.run(&["config", "paths", "--doc-types"])?;
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 13);
    assert!(String::from_utf8_lossy(&output.stderr).contains("is blank"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_doc_types_refuses_an_unsafe_path_with_empty_stdout() -> TestResult {
    let workspace = workspace("doc-type-escape")?;
    let output = run_in(&workspace, &["config", "paths", "--doc-types"])?;
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe path"));
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_doc_types_stays_fail_closed_on_escape_with_fail_safe() -> TestResult {
    let workspace = workspace("doc-type-escape")?;
    let output = run_in(
        &workspace,
        &["config", "paths", "--doc-types", "--fail-safe"],
    )?;
    assert!(output.stdout.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn dump_matches_the_committed_golden() -> TestResult {
    let workspace = workspace("dump")?;
    let output = run_in(&workspace, &["config", "dump"])?;
    assert_eq!(output.stdout, golden("dump", "dump.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn dump_hides_credential_values() -> TestResult {
    let workspace = workspace("dump")?;
    let output = run_in(&workspace, &["config", "dump"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("`jira.token` | *(set — hidden)*"));
    assert!(!stdout.contains("secret-value"));
    Ok(())
}

#[test]
fn dump_of_an_unconfigured_repo_prints_nothing() -> TestResult {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "config-read-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join(".git"))?;
    let output = run_in(&root, &["config", "dump"])?;
    assert!(output.stdout.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn dump_with_fail_safe_renders_the_unavailable_notice() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "dump", "--fail-safe"])?;
    assert_eq!(output.stdout, b"## Effective Configuration Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn review_matches_the_baseline_goldens_for_every_mode() -> TestResult {
    for mode in ["pr", "plan", "work-item"] {
        let workspace = workspace("baseline")?;
        let output = run_in(&workspace, &["config", "review", mode])?;
        assert_eq!(
            output.stdout,
            golden("baseline", &format!("review-{mode}.golden"))?,
            "review {mode} drifted from its golden"
        );
        assert_eq!(code(&output), 0);
    }
    Ok(())
}

#[test]
fn review_lists_a_custom_lens_only_in_its_applies_to_modes() -> TestResult {
    let workspace = workspace("custom-lenses")?;

    let pr = run_in(&workspace, &["config", "review", "pr"])?;
    let pr_out = String::from_utf8_lossy(&pr.stdout);
    assert!(pr_out.contains("| perf-custom |"));
    assert!(!pr_out.contains("| wi-custom |"));

    let wi = run_in(&workspace, &["config", "review", "work-item"])?;
    let wi_out = String::from_utf8_lossy(&wi.stdout);
    assert!(wi_out.contains("| wi-custom |"));
    assert!(!wi_out.contains("| perf-custom |"));
    Ok(())
}

#[test]
fn a_custom_lens_row_uses_a_single_slash_path_and_the_right_source(
) -> TestResult {
    let workspace = workspace("custom-lenses")?;
    let output = run_in(&workspace, &["config", "review", "pr"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Divergence 2: the bash double slash is fixed here.
    assert!(stdout.contains("/lenses/perf-lens/SKILL.md | custom |"));
    assert!(!stdout.contains("/lenses/perf-lens//SKILL.md"));
    // auto_detect present → "custom"; wi-custom has none → "always include".
    let wi = run_in(&workspace, &["config", "review", "work-item"])?;
    let wi_out = String::from_utf8_lossy(&wi.stdout);
    assert!(wi_out.contains("| wi-custom |"));
    assert!(wi_out.contains("| custom (always include) |"));
    Ok(())
}

#[test]
fn review_with_fail_safe_renders_the_unavailable_notice() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "review", "pr", "--fail-safe"])?;
    assert_eq!(output.stdout, b"## Review Configuration Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn a_review_without_a_mode_is_a_usage_error() -> TestResult {
    let workspace = workspace("baseline")?;
    let output = run_in(&workspace, &["config", "review"])?;
    assert_eq!(code(&output), 1);
    Ok(())
}

#[test]
fn summary_matches_the_committed_golden() -> TestResult {
    let workspace = workspace("summary")?;
    let output = run_in(&workspace, &["config", "summary"])?;
    assert_eq!(output.stdout, golden("summary", "summary.golden")?);
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn summary_hook_wraps_the_plain_output_as_additional_context() -> TestResult {
    let workspace = workspace("summary")?;
    let plain = run_in(&workspace, &["config", "summary"])?;
    let hook = run_in(&workspace, &["config", "summary", "--format", "hook"])?;
    let hook_out = String::from_utf8_lossy(&hook.stdout);
    let plain_out = String::from_utf8_lossy(&plain.stdout);
    let plain_trimmed = plain_out.trim_end_matches('\n');
    assert!(hook_out.starts_with(
        "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\","
    ));
    // The additionalContext is the plain summary with newlines JSON-escaped.
    let escaped = plain_trimmed.replace('\n', "\\n");
    assert!(hook_out.contains(&escaped));
    assert_eq!(code(&hook), 0);
    Ok(())
}

#[test]
fn summary_of_an_initialised_repo_with_no_config_prints_nothing() -> TestResult
{
    let workspace = workspace("empty-summary")?;
    let plain = run_in(&workspace, &["config", "summary"])?;
    assert!(plain.stdout.is_empty());
    assert_eq!(code(&plain), 0);
    let hook = run_in(&workspace, &["config", "summary", "--format", "hook"])?;
    assert!(hook.stdout.is_empty());
    assert_eq!(code(&hook), 0);
    Ok(())
}

#[test]
fn summary_of_an_uninitialised_repo_prints_the_init_hint() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run(&["config", "summary"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("has not been initialised"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn summary_with_fail_safe_suppresses_a_read_failure() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "summary", "--fail-safe"])?;
    assert!(output.stdout.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn summary_hook_without_fail_safe_fails_loud_on_a_read_failure() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "summary", "--format", "hook"])?;
    assert!(output.stdout.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

/// The committed fixture plugin root carrying `templates/`.
fn plugin_root() -> PathBuf {
    PathBuf::from(FIXTURES).join("plugin")
}

fn run_with_plugin(
    cwd: &Path,
    args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator"));
    command.current_dir(cwd);
    command.env_remove("ACCELERATOR_LOG");
    command.env_remove("ACCELERATOR_CACHE_DIR");
    command.env_remove("ACCELERATOR_RELEASE_BASE_URL");
    command.env("CLAUDE_PLUGIN_ROOT", plugin_root());
    command.args(args);
    Ok(command.output()?)
}

#[test]
fn template_wraps_the_plugin_default_in_markdown_fences() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output =
        run_with_plugin(&fixture.root, &["config", "template", "demo"])?;
    assert_eq!(
        output.stdout,
        b"```markdown\n# Demo Template\n\nBody line.\n```\n"
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn template_not_found_fails_closed_even_with_fail_safe() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "template", "nonesuch", "--fail-safe"],
    )?;
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not found"));
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn templates_show_prints_the_source_header_and_content() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "show", "demo"],
    )?;
    assert_eq!(
        output.stdout,
        b"Source: plugin default (<plugin>/templates/demo.md)\n---\n\
          # Demo Template\n\nBody line.\n"
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn templates_list_tabulates_every_plugin_template() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output =
        run_with_plugin(&fixture.root, &["config", "templates", "list"])?;
    assert_eq!(
        output.stdout,
        b"| Template | Source | Path |\n\
          |----------|--------|------|\n\
          | `demo` | plugin default | `<plugin>/templates/demo.md` |\n\
          | `other` | plugin default | `<plugin>/templates/other.md` |\n"
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn a_user_override_template_wins_over_the_plugin_default() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    fs::create_dir_all(fixture.root.join(".accelerator/templates"))?;
    fs::write(
        fixture.root.join(".accelerator/templates/demo.md"),
        "# Overridden\n",
    )?;
    let output =
        run_with_plugin(&fixture.root, &["config", "template", "demo"])?;
    assert_eq!(output.stdout, b"```markdown\n# Overridden\n```\n");
    Ok(())
}

#[test]
fn an_already_fenced_template_is_not_double_wrapped() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    fs::create_dir_all(fixture.root.join(".accelerator/templates"))?;
    fs::write(
        fixture.root.join(".accelerator/templates/demo.md"),
        "```markdown\nalready fenced\n```\n",
    )?;
    let output =
        run_with_plugin(&fixture.root, &["config", "template", "demo"])?;
    assert_eq!(output.stdout, b"```markdown\nalready fenced\n```\n");
    Ok(())
}

#[test]
fn a_traversing_template_name_is_refused() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output =
        run_with_plugin(&fixture.root, &["config", "template", "../../etc"])?;
    assert!(output.stdout.is_empty());
    assert_ne!(code(&output), 0);
    Ok(())
}

const OVERRIDDEN: &str = "---\ncore:\n  key: teamval\n---\n";
const OVERRIDE_LOCAL: &str = "---\ncore:\n  key: personalval\n---\n";

#[test]
fn explain_names_both_files_and_attributes_to_personal() -> TestResult {
    let fixture = Fixture::new()?.team(OVERRIDDEN)?;
    fs::write(
        fixture.root.join(".accelerator/config.local.md"),
        OVERRIDE_LOCAL,
    )?;
    let with = fixture.run(&["config", "get", "core.key", "--explain"])?;
    let without = fixture.run(&["config", "get", "core.key"])?;
    // stdout is byte-identical with and without --explain.
    assert_eq!(with.stdout, without.stdout);
    assert_eq!(with.stdout, b"personalval\n");
    let stderr = String::from_utf8_lossy(&with.stderr);
    assert!(stderr.contains(".accelerator/config.md"));
    assert!(stderr.contains(".accelerator/config.local.md"));
    assert!(stderr.contains("resolved from: personal"));
    // Without --explain there is no provenance.
    assert!(without.stderr.is_empty());
    Ok(())
}

#[test]
fn config_help_lists_every_subcommand() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run(&["config", "--help"])?;
    assert_eq!(code(&output), 0);
    let help = String::from_utf8_lossy(&output.stdout);
    for subcommand in [
        "get",
        "path",
        "agent",
        "agents",
        "work",
        "context",
        "instructions",
        "paths",
        "dump",
        "review",
        "summary",
        "template",
        "templates",
    ] {
        assert!(
            help.contains(subcommand),
            "config --help omits `{subcommand}`"
        );
    }
    Ok(())
}

#[test]
fn every_subcommand_help_renders_its_contract() -> TestResult {
    let cases: &[(&[&str], &str)] = &[
        (&["get", "--help"], "Print a configuration value"),
        (&["path", "--help"], "Print a configured"),
        (&["agent", "--help"], "Print an agent-name override"),
        (&["agents", "--help"], "## Agent Names"),
        (&["work", "--help"], "work.integration"),
        (&["context", "--help"], "## Project Context"),
        (&["instructions", "--help"], "## Additional Instructions"),
        (&["paths", "--help"], "## Configured Paths"),
        (&["dump", "--help"], "## Effective Configuration"),
        (&["review", "--help"], "## Review Configuration"),
        (&["summary", "--help"], "SessionStart"),
        (&["template", "--help"], "markdown fences"),
        (&["templates", "list", "--help"], "resolution source"),
        (&["templates", "show", "--help"], "source metadata"),
    ];
    for (args, expected) in cases {
        let fixture = Fixture::new()?;
        let mut full = vec!["config"];
        full.extend_from_slice(args);
        let output = fixture.run(&full)?;
        assert_eq!(code(&output), 0, "help exit for {args:?}");
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(
            help.contains(expected),
            "`config {args:?}` help omits its contract text {expected:?}"
        );
    }
    Ok(())
}

#[test]
fn summary_resolves_the_init_sentinel_against_the_project_root() -> TestResult {
    // Divergence 3: from a subdirectory the sentinel is found via the project
    // root, so an initialised repo is not misreported as uninitialised.
    let workspace = workspace("empty-summary")?;
    let deep = workspace.join("src/deep");
    fs::create_dir_all(&deep)?;
    let output = run_in(&deep, &["config", "summary"])?;
    assert!(output.stdout.is_empty(), "should read as initialised");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn summary_hook_with_fail_safe_suppresses_a_read_failure() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&[
        "config",
        "summary",
        "--format",
        "hook",
        "--fail-safe",
    ])?;
    assert!(output.stdout.is_empty());
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_degrades_the_project_source_and_keeps_the_skill_block() -> TestResult
{
    let workspace = workspace("context-full")?;
    // Make config.md unreadable (a directory), so the project body read fails
    // while the skill context file is untouched.
    fs::remove_file(workspace.join(".accelerator/config.md"))?;
    fs::remove_file(workspace.join(".accelerator/config.local.md"))?;
    fs::create_dir(workspace.join(".accelerator/config.md"))?;
    let output = run_in(
        &workspace,
        &["config", "context", "--skill", "demo", "--fail-safe"],
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("## Project Context Unavailable"));
    assert!(stdout.contains("## Skill-Specific Context\n"));
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn instructions_with_fail_safe_renders_the_unavailable_notice() -> TestResult {
    // An invalid skill name degrades to the notice under --fail-safe.
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let output =
        fixture.run(&["config", "instructions", "../bad", "--fail-safe"])?;
    assert_eq!(output.stdout, b"## Skill Instructions Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn paths_with_fail_safe_renders_the_unavailable_notice() -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = fixture.run(&["config", "paths", "--fail-safe"])?;
    assert_eq!(output.stdout, b"## Configured Paths Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn template_with_fail_safe_renders_the_unavailable_notice_on_a_read_failure(
) -> TestResult {
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "template", "demo", "--fail-safe"],
    )?;
    assert_eq!(output.stdout, b"## Template Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn templates_list_with_fail_safe_renders_the_unavailable_notice() -> TestResult
{
    let fixture = Fixture::new()?.team(MALFORMED)?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "list", "--fail-safe"],
    )?;
    assert_eq!(output.stdout, b"## Template Unavailable\n");
    assert_eq!(code(&output), 0);
    Ok(())
}

#[test]
fn context_both_sources_failing_prints_both_notices_with_one_blank_line(
) -> TestResult {
    let workspace = workspace("context-full")?;
    // Make both the config body and the skill context unreadable (directories).
    fs::remove_file(workspace.join(".accelerator/config.md"))?;
    fs::remove_file(workspace.join(".accelerator/config.local.md"))?;
    fs::create_dir(workspace.join(".accelerator/config.md"))?;
    fs::remove_file(workspace.join(".accelerator/skills/demo/context.md"))?;
    fs::create_dir(workspace.join(".accelerator/skills/demo/context.md"))?;
    let output = run_in(
        &workspace,
        &["config", "context", "--skill", "demo", "--fail-safe"],
    )?;
    assert_eq!(
        output.stdout,
        b"## Project Context Unavailable\n\n## Skill-Specific Context Unavailable\n"
    );
    assert_eq!(code(&output), 0);
    Ok(())
}

// --- config set (write path) ---

fn read_file(root: &Path, name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(root.join(".accelerator").join(name))?)
}

#[test]
fn set_round_trips_and_preserves_the_body() -> TestResult {
    let fixture = Fixture::new()?
        .team("---\ncore:\n  example: old\n---\nBody prose.\n\nMore.\n")?;
    let output = fixture.run(&[
        "config",
        "set",
        "core.example",
        "new",
        "--level",
        "team",
    ])?;
    assert_eq!(code(&output), 0);
    assert!(output.stdout.is_empty());
    let after = read_file(&fixture.root, "config.md")?;
    assert!(after.contains("example: new"));
    assert!(
        after.ends_with("Body prose.\n\nMore.\n"),
        "body lost: {after:?}"
    );
    let reread =
        fixture.run(&["config", "get", "core.example", "--level", "team"])?;
    assert_eq!(reread.stdout, b"new\n");
    Ok(())
}

#[test]
fn set_writes_the_team_level_when_asked() -> TestResult {
    let fixture = Fixture::new()?;
    let output =
        fixture.run(&["config", "set", "core.k", "v", "--level", "team"])?;
    assert_eq!(code(&output), 0);
    assert!(fixture.root.join(".accelerator/config.md").is_file());
    Ok(())
}

#[test]
fn set_gitignores_config_local_on_a_fresh_repo() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.run(&["config", "set", "jira.token", "secret"])?;
    let gitignore =
        fs::read_to_string(fixture.root.join(".accelerator/.gitignore"))?;
    assert!(gitignore.lines().any(|line| line == "config.local.md"));
    Ok(())
}

#[test]
fn set_ensures_the_rule_on_a_gitignore_lacking_it() -> TestResult {
    let fixture = Fixture::new()?;
    fs::create_dir_all(fixture.root.join(".accelerator"))?;
    fs::write(fixture.root.join(".accelerator/.gitignore"), "other-rule\n")?;
    fixture.run(&["config", "set", "jira.token", "secret"])?;
    let gitignore =
        fs::read_to_string(fixture.root.join(".accelerator/.gitignore"))?;
    assert!(gitignore.lines().any(|line| line == "other-rule"));
    assert!(gitignore.lines().any(|line| line == "config.local.md"));
    Ok(())
}

#[test]
fn set_a_deeply_nested_key_round_trips() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.run(&["config", "set", "a.b.c.d", "deep", "--level", "team"])?;
    let output = fixture.run(&["config", "get", "a.b.c.d"])?;
    assert_eq!(output.stdout, b"deep\n");
    Ok(())
}

const MALFORMED_UNTERMINATED: &str = "---\nkey: value\n";
const MALFORMED_YAML: &str = "---\nkey: : :\n  - broken\n---\nbody\n";
const MALFORMED_FLOW: &str = "---\nkey: [1, 2\n---\nbody\n";
const NON_MAPPING_ROOT: &str = "---\n- a\n- b\n---\nbody\n";

#[test]
fn set_refuses_a_malformed_file_and_leaves_it_byte_identical() -> TestResult {
    for content in [MALFORMED_UNTERMINATED, MALFORMED_YAML, MALFORMED_FLOW] {
        assert_set_refuses_and_preserves(content)?;
    }
    Ok(())
}

#[test]
fn set_refuses_a_non_mapping_root_and_leaves_it_byte_identical() -> TestResult {
    assert_set_refuses_and_preserves(NON_MAPPING_ROOT)
}

fn assert_set_refuses_and_preserves(content: &str) -> TestResult {
    let fixture = Fixture::new()?.team(content)?;
    let output =
        fixture.run(&["config", "set", "core.k", "v", "--level", "team"])?;
    assert_ne!(code(&output), 0, "should refuse: {content:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(
        read_file(&fixture.root, "config.md")?,
        content,
        "file must be left untouched: {content:?}"
    );
    Ok(())
}

#[test]
fn set_refuses_a_config_dir_symlink_escape() -> TestResult {
    let fixture = Fixture::new()?;
    let outside = fixture.root.join("outside.md");
    fs::write(&outside, "---\njira:\n  token: original\n---\n")?;
    fs::create_dir_all(fixture.root.join(".accelerator"))?;
    std::os::unix::fs::symlink(
        &outside,
        fixture.root.join(".accelerator/config.local.md"),
    )?;
    let output = fixture.run(&["config", "set", "jira.token", "stolen"])?;
    assert_ne!(code(&output), 0);
    assert_eq!(
        fs::read_to_string(&outside)?,
        "---\njira:\n  token: original\n---\n",
        "the symlink target must not be clobbered"
    );
    Ok(())
}

#[test]
fn set_against_an_unwritable_config_fails_non_zero() -> TestResult {
    use std::os::unix::fs::PermissionsExt as _;
    let fixture = Fixture::new()?;
    let dir = fixture.root.join(".accelerator");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("config.md"), "---\ncore:\n  k: old\n---\n")?;
    // Make the config directory unwritable so the atomic rename cannot land.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500))?;
    let output =
        fixture.run(&["config", "set", "core.k", "new", "--level", "team"])?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755))?;
    assert_ne!(code(&output), 0);
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn set_rejects_the_allow_legacy_layout_flag() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run(&[
        "config",
        "set",
        "--allow-legacy-layout",
        "core.k",
        "v",
    ])?;
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn set_leaves_no_temp_file_behind() -> TestResult {
    let fixture =
        Fixture::new()?.team("---\ncore:\n  example: old\n---\nBody.\n")?;
    fixture.run(&[
        "config",
        "set",
        "core.example",
        "new",
        "--level",
        "team",
    ])?;
    let leftovers: Vec<_> = fs::read_dir(fixture.root.join(".accelerator"))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with(".tmp-")
        })
        .collect();
    assert!(leftovers.is_empty(), "temp file left: {leftovers:?}");
    Ok(())
}

// --- templates eject / diff / reset (write path) ---

/// Seeds a user-directory override for `demo`, making it "already customised"
/// for every subcommand.
fn customise_demo(root: &Path, body: &str) -> TestResult {
    fs::create_dir_all(root.join(".accelerator/templates"))?;
    fs::write(root.join(".accelerator/templates/demo.md"), body)?;
    Ok(())
}

#[test]
fn eject_writes_the_default_when_not_customised() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert_eq!(
        output.stdout,
        b"Ejected: demo -> .accelerator/templates/demo.md\n"
    );
    let written = fs::read_to_string(
        fixture.root.join(".accelerator/templates/demo.md"),
    )?;
    assert_eq!(written, "# Demo Template\n\nBody line.\n");
    Ok(())
}

#[test]
fn eject_against_already_customised_exits_two() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "demo"],
    )?;
    assert_eq!(code(&output), 2);
    assert_eq!(
        fs::read_to_string(
            fixture.root.join(".accelerator/templates/demo.md")
        )?,
        "# Mine\n",
        "existing file must be left untouched"
    );
    Ok(())
}

#[test]
fn eject_force_overwrites_an_existing_file() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "--force", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert_eq!(
        output.stdout,
        b"Overwritten: demo -> .accelerator/templates/demo.md\n"
    );
    assert_eq!(
        fs::read_to_string(
            fixture.root.join(".accelerator/templates/demo.md")
        )?,
        "# Demo Template\n\nBody line.\n"
    );
    Ok(())
}

#[test]
fn eject_dry_run_reports_without_writing() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "--dry-run", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Would eject:"));
    assert!(!fixture.root.join(".accelerator/templates/demo.md").exists());
    Ok(())
}

#[test]
fn eject_dry_run_on_an_existing_file_would_skip_and_exits_two() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "--dry-run", "demo"],
    )?;
    assert_eq!(code(&output), 2);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Would skip:"));
    Ok(())
}

#[test]
fn eject_all_writes_every_template() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "--all"],
    )?;
    assert_eq!(code(&output), 0);
    for name in ["demo", "other"] {
        assert!(
            fixture
                .root
                .join(format!(".accelerator/templates/{name}.md"))
                .is_file(),
            "{name} not ejected"
        );
    }
    Ok(())
}

#[test]
fn eject_all_with_a_conflict_exits_two_but_writes_the_rest() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "--all"],
    )?;
    assert_eq!(code(&output), 2);
    assert!(fixture
        .root
        .join(".accelerator/templates/other.md")
        .is_file());
    Ok(())
}

#[test]
fn eject_of_an_unknown_template_exits_one() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "missing"],
    )?;
    assert_eq!(code(&output), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Available:"));
    Ok(())
}

#[test]
fn eject_respects_the_paths_templates_override() -> TestResult {
    let fixture =
        Fixture::new()?.team("---\npaths:\n  templates: docs/tpl\n---\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert!(fixture.root.join("docs/tpl/demo.md").is_file());
    Ok(())
}

#[test]
fn eject_of_a_traversing_name_is_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "../../etc/passwd"],
    )?;
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn diff_against_not_customised_exits_two() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "demo"],
    )?;
    assert_eq!(code(&output), 2);
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("No customised template found"));
    Ok(())
}

#[test]
fn diff_against_customised_shows_the_addition() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(
        &fixture.root,
        "# Demo Template\n\nBody line.\n\n# Extra\n",
    )?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Comparing plugin default vs user override:"));
    assert!(stdout.contains("+# Extra"));
    Ok(())
}

#[test]
fn diff_of_an_identical_override_reports_identical() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Demo Template\n\nBody line.\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("Templates are identical."));
    Ok(())
}

#[test]
fn diff_of_an_unknown_template_exits_one() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "missing"],
    )?;
    assert_eq!(code(&output), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Available:"));
    Ok(())
}

#[test]
fn diff_of_a_traversing_name_is_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "../../etc/passwd"],
    )?;
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn reset_against_not_customised_exits_two() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "demo"],
    )?;
    assert_eq!(code(&output), 2);
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("already using plugin default"));
    Ok(())
}

#[test]
fn reset_reports_the_override_without_confirm() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Found override: user override"));
    assert!(stdout.contains(".accelerator/templates/demo.md"));
    assert!(
        fixture
            .root
            .join(".accelerator/templates/demo.md")
            .is_file(),
        "reset without --confirm must not delete"
    );
    Ok(())
}

#[test]
fn reset_confirm_deletes_the_override() -> TestResult {
    let fixture = Fixture::new()?;
    customise_demo(&fixture.root, "# Mine\n")?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "--confirm", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert_eq!(output.stdout, b"Reset: demo\n");
    assert!(!fixture.root.join(".accelerator/templates/demo.md").exists());
    Ok(())
}

#[test]
fn reset_of_an_unknown_template_exits_one() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "missing"],
    )?;
    assert_eq!(code(&output), 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Available:"));
    Ok(())
}

#[test]
fn reset_of_a_traversing_name_is_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "../../etc/passwd"],
    )?;
    assert_ne!(code(&output), 0);
    Ok(())
}

#[test]
fn a_config_path_override_is_customised_for_diff_and_reset_but_not_eject(
) -> TestResult {
    let fixture = Fixture::new()?
        .team("---\ntemplates:\n  demo: custom/my-demo.md\n---\n")?;
    fs::create_dir_all(fixture.root.join("custom"))?;
    fs::write(
        fixture.root.join("custom/my-demo.md"),
        "# Demo Template\n\nBody line.\n\n# Config addition\n",
    )?;

    let eject = run_with_plugin(
        &fixture.root,
        &["config", "templates", "eject", "demo"],
    )?;
    assert_eq!(code(&eject), 0, "eject ignores the config-path override");
    assert!(fixture
        .root
        .join(".accelerator/templates/demo.md")
        .is_file());

    let diff = run_with_plugin(
        &fixture.root,
        &["config", "templates", "diff", "demo"],
    )?;
    assert_eq!(code(&diff), 0);

    let reset = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "demo"],
    )?;
    assert_eq!(code(&reset), 0);
    assert!(String::from_utf8_lossy(&reset.stdout)
        .contains("also remove the 'templates.demo' entry"));
    Ok(())
}

#[test]
fn reset_warns_when_the_override_is_outside_the_project() -> TestResult {
    let outside = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "config-outside-{}-{}.md",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&outside, "# Outside\n")?;
    let fixture = Fixture::new()?.team(&format!(
        "---\ntemplates:\n  demo: {}\n---\n",
        outside.display()
    ))?;
    let output = run_with_plugin(
        &fixture.root,
        &["config", "templates", "reset", "demo"],
    )?;
    assert_eq!(code(&output), 0);
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("outside the project directory"));
    Ok(())
}

// --- config init ---

#[test]
fn init_creates_the_documented_tree() -> TestResult {
    let fixture = Fixture::new()?;
    let output = fixture.run(&["config", "init"])?;
    assert_eq!(code(&output), 0);
    assert!(output.stdout.is_empty(), "init is silent on stdout");
    for dir in [
        "meta/plans",
        "meta/research/codebase",
        "meta/decisions",
        "meta/prs",
        "meta/validations",
        "meta/reviews/plans",
        "meta/reviews/prs",
        "meta/reviews/work",
        "meta/work",
        "meta/notes",
        "meta/research/design-inventories",
        "meta/research/design-gaps",
        "meta/global",
        "meta/research/issues",
    ] {
        assert!(
            fixture.root.join(dir).join(".gitkeep").is_file(),
            "{dir}/.gitkeep missing"
        );
    }
    for dir in ["state", "skills", "lenses", "templates"] {
        assert!(fixture
            .root
            .join(".accelerator")
            .join(dir)
            .join(".gitkeep")
            .is_file());
    }
    let inner =
        fs::read_to_string(fixture.root.join(".accelerator/.gitignore"))?;
    assert!(inner.lines().any(|line| line == "config.local.md"));
    assert!(inner.lines().any(|line| line == ".tmp-*"));
    assert_eq!(
        fs::read_to_string(fixture.root.join(".accelerator/tmp/.gitignore"))?,
        "*\n!.gitkeep\n!.gitignore\n"
    );
    let root_ignore = fs::read_to_string(fixture.root.join(".gitignore"))?;
    assert!(root_ignore
        .lines()
        .any(|line| line == ".accelerator/config.local.md"));
    Ok(())
}

#[test]
fn init_is_idempotent() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.run(&["config", "init"])?;
    let output = fixture.run(&["config", "init"])?;
    assert_eq!(code(&output), 0);
    let root_ignore = fs::read_to_string(fixture.root.join(".gitignore"))?;
    assert_eq!(
        root_ignore
            .lines()
            .filter(|l| *l == ".accelerator/config.local.md")
            .count(),
        1,
        "the root rule must not be duplicated"
    );
    Ok(())
}
