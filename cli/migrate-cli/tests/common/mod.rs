//! Shared helpers for the suites that drive `accelerator-migrate` over real
//! git and jj repositories under a hermetic environment.

#![allow(dead_code)]

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::stalled_migration::corpus::migrate_command;
use vcs_test_support::stalled_migration::corpus::work_item;
use vcs_test_support::stalled_migration::corpus::MIGRATIONS_BEFORE_0007;

pub type TestError = Box<dyn std::error::Error>;

pub const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

pub const MIGRATION_0007: &str = "0007-unify-meta-corpus-frontmatter";

pub const RUN_BASE_FILE: &str = ".accelerator/state/migrations-run.id";

pub const MANIFEST_FILE: &str = ".accelerator/state/migrations-run-paths.txt";

const MIGRATIONS_FROM_0007: [&str; 5] = [
    MIGRATION_0007,
    "0008-canonical-frontmatter-quoting",
    "0009-split-work-key-from-tracker-scope-key",
    "0010-strip-research-title-prefix",
    "0011-ignore-machine-local-sync-state",
];

pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

pub fn tempdir(tag: &str) -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("migrate-{tag}-"))
        .tempdir()?)
}

pub fn write(
    root: &Path,
    relative: &str,
    content: &str,
) -> Result<(), TestError> {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

/// Runs the binary under `env`, as [`migrate_command`] does.
pub fn run(
    env: &Hermetic,
    root: &Path,
    args: &[&str],
    env_extra: &[(&str, &str)],
) -> Result<Outcome, TestError> {
    let mut command = migrate_command(Path::new(BIN), root, env);
    command.args(args);
    for (key, value) in env_extra {
        command.env(key, value);
    }
    let output = command.output()?;
    Ok(Outcome {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        code: output.status.code().unwrap_or(-1),
    })
}

pub fn mark_all_migrations_applied(root: &Path) -> Result<(), TestError> {
    let ledger: String = MIGRATIONS_BEFORE_0007
        .iter()
        .chain(MIGRATIONS_FROM_0007.iter())
        .flat_map(|id| [*id, "\n"])
        .collect();
    write(root, ".accelerator/state/migrations-applied", &ledger)
}

pub fn mark_migrations_before_0007_applied(
    root: &Path,
) -> Result<(), TestError> {
    let ledger: String = MIGRATIONS_BEFORE_0007
        .iter()
        .flat_map(|id| [*id, "\n"])
        .collect();
    write(root, ".accelerator/state/migrations-applied", &ledger)
}

/// One work item whose reference to another resolves ambiguously, so 0007
/// stalls on it when no decision input is available.
pub fn seed_ambiguous_reference(root: &Path) -> Result<(), TestError> {
    write(
        root,
        "meta/work/0042-target.md",
        &work_item("0042", "Target", ""),
    )?;
    write(
        root,
        "meta/work/0001-source.md",
        &work_item(
            "0001",
            "Source",
            "\n## References\n- `meta/work/0042-target.md`\n",
        ),
    )?;
    mark_migrations_before_0007_applied(root)
}

pub fn decisions_file_for(migration_id: &str) -> String {
    format!(".accelerator/state/migrations-{migration_id}-decisions.txt")
}

pub fn applied_ledger(root: &Path) -> Result<String, TestError> {
    Ok(fs::read_to_string(
        root.join(".accelerator/state/migrations-applied"),
    )?)
}
