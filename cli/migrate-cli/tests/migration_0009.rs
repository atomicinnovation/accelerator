//! Migration 0009 (`split-work-key-from-tracker-scope-key`) driven end to end
//! against the compiled binary, with the ledger pre-seeded through 0008 so 0009
//! is the sole pending migration.
#![allow(clippy::literal_string_with_formatting_args)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

const LEDGER_THROUGH_0008: &str = "0001-rename-tickets-to-work\n\
     0002-rename-work-items-with-project-prefix\n\
     0003-relocate-accelerator-state\n\
     0004-restructure-meta-research-into-subject-subcategories\n\
     0005-rename-work-item-type-to-kind\n\
     0006-canonicalise-work-item-id-and-author\n\
     0007-unify-meta-corpus-frontmatter\n\
     0008-canonical-frontmatter-quoting\n";

fn write(dir: &Path, relative: &str, content: &str) -> Result<(), TestError> {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn scratch(config: &str) -> Result<TempDir, TestError> {
    let dir = TempDir::new()?;
    write(dir.path(), ".accelerator/config.md", config)?;
    write(
        dir.path(),
        ".accelerator/state/migrations-applied",
        LEDGER_THROUGH_0008,
    )?;
    Ok(dir)
}

fn run(root: &Path) -> Result<std::process::Output, TestError> {
    Ok(Command::new(BIN).current_dir(root).output()?)
}

fn config(root: &Path) -> Result<String, TestError> {
    Ok(fs::read_to_string(root.join(".accelerator/config.md"))?)
}

#[test]
fn a_tracker_backed_legacy_config_materialises_both_keys(
) -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  integration: jira\n  \
         id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: \"PP\"\njira:\n  site: acme\n---\n",
    )?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    let config = config(dir.path())?;
    assert!(
        config.contains("id_pattern: \"{key}-{number:04d}\""),
        "{config}"
    );
    assert!(config.contains("key: \"PP\""), "{config}");
    assert!(config.contains("project_key: \"PP\""), "{config}");
    assert!(!config.contains("default_project_code"), "{config}");

    assert!(dir.path().join(".accelerator/config.md.0009.bak").exists());
    let applied = fs::read_to_string(
        dir.path().join(".accelerator/state/migrations-applied"),
    )?;
    assert!(
        applied.contains("0009-split-work-key-from-tracker-scope-key"),
        "{applied}"
    );
    Ok(())
}

#[test]
fn a_tracker_less_legacy_config_materialises_only_work_key(
) -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: \"PP\"\n---\n",
    )?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    let config = config(dir.path())?;
    assert!(config.contains("key: \"PP\""), "{config}");
    assert!(!config.contains("project_key"), "{config}");
    assert!(!config.contains("team_key"), "{config}");
    assert!(!config.contains("default_project_code"), "{config}");
    Ok(())
}

#[test]
fn a_bare_numeric_tracker_config_materialises_no_work_key(
) -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  integration: jira\n  \
         id_pattern: \"{number:04d}\"\n  \
         default_project_code: \"PP\"\njira:\n  site: acme\n---\n",
    )?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    let config = config(dir.path())?;
    assert!(config.contains("project_key: \"PP\""), "{config}");
    assert!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("key:")),
        "no work.key materialised: {config}"
    );
    assert!(!config.contains("default_project_code"), "{config}");
    Ok(())
}

#[test]
fn a_second_run_is_a_no_op_and_the_nag_is_silent() -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  integration: jira\n  \
         id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: \"PP\"\njira:\n  site: acme\n---\n",
    )?;
    run(dir.path())?;
    let after_first = config(dir.path())?;

    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        String::from_utf8(out.stdout)?.contains("No pending migrations."),
        "the ledger advanced, so a second run has nothing pending"
    );
    assert_eq!(config(dir.path())?, after_first);
    Ok(())
}

#[test]
fn a_clean_repo_still_advances_the_ledger() -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n  key: \"PP\"\n---\n",
    )?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let applied = fs::read_to_string(
        dir.path().join(".accelerator/state/migrations-applied"),
    )?;
    assert!(
        applied.contains("0009-split-work-key-from-tracker-scope-key"),
        "a repo with nothing to migrate still advances past 0009: {applied}"
    );
    Ok(())
}

#[test]
fn a_divergent_pinned_pair_aborts_without_mutating() -> Result<(), TestError> {
    let original = "---\nwork:\n  key: \"AA\"\n  \
         default_project_code: \"BB\"\n---\n";
    let dir = scratch(original)?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8(out.stderr)?.contains("mixed-state"),
        "the refusal names the mixed state"
    );
    assert_eq!(config(dir.path())?, original);
    assert!(!dir.path().join(".accelerator/config.md.0009.bak").exists());
    Ok(())
}

#[test]
fn a_legacy_beside_a_correct_scope_key_removes_the_redundant_key(
) -> Result<(), TestError> {
    let dir = scratch(
        "---\nwork:\n  integration: jira\n  \
         id_pattern: \"{number:04d}\"\n  \
         default_project_code: \"PP\"\njira:\n  \
         project_key: \"OPS\"\n---\n",
    )?;
    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let config = config(dir.path())?;
    assert!(config.contains("project_key: \"OPS\""), "{config}");
    assert!(!config.contains("project_key: \"PP\""), "{config}");
    assert!(!config.contains("default_project_code"), "{config}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn the_personal_file_and_its_backup_stay_owner_only() -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new()?;
    write(
        dir.path(),
        ".accelerator/config.md",
        "---\nwork:\n  integration: jira\n  \
         id_pattern: \"{project}-{number:04d}\"\njira:\n  site: acme\n---\n",
    )?;
    write(
        dir.path(),
        ".accelerator/config.local.md",
        "---\nwork:\n  default_project_code: \"PP\"\n---\n",
    )?;
    fs::set_permissions(
        dir.path().join(".accelerator/config.local.md"),
        fs::Permissions::from_mode(0o600),
    )?;
    write(
        dir.path(),
        ".accelerator/state/migrations-applied",
        LEDGER_THROUGH_0008,
    )?;

    let out = run(dir.path())?;
    assert_eq!(out.status.code(), Some(0), "{out:?}");

    let mode_of = |relative: &str| -> Result<u32, TestError> {
        Ok(fs::metadata(dir.path().join(relative))?
            .permissions()
            .mode()
            & 0o777)
    };
    assert_eq!(mode_of(".accelerator/config.local.md")?, 0o600);
    assert_eq!(mode_of(".accelerator/config.local.md.0009.bak")?, 0o600);
    Ok(())
}
