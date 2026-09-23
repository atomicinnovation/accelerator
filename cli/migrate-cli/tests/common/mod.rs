//! Shared helpers for the suites that drive `accelerator-migrate` over real
//! git and jj repositories under a hermetic environment.

#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;

pub type TestError = Box<dyn std::error::Error>;

pub const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

pub const MIGRATION_0007: &str = "0007-unify-meta-corpus-frontmatter";

pub const RUN_BASE_FILE: &str = ".accelerator/state/migrations-run.id";

pub const MANIFEST_FILE: &str = ".accelerator/state/migrations-run-paths.txt";

const MIGRATIONS_BEFORE_0007: [&str; 6] = [
    "0001-rename-tickets-to-work",
    "0002-rename-work-items-with-project-prefix",
    "0003-relocate-accelerator-state",
    "0004-restructure-meta-research-into-subject-subcategories",
    "0005-rename-work-item-type-to-kind",
    "0006-canonicalise-work-item-id-and-author",
];

const MIGRATIONS_FROM_0007: [&str; 4] = [
    MIGRATION_0007,
    "0008-canonical-frontmatter-quoting",
    "0009-split-work-key-from-tracker-scope-key",
    "0010-strip-research-title-prefix",
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

/// Runs the binary under `env`, with no stdin and warnings logged, so an
/// assertion on the absence of a `WARN` line is never vacuous.
pub fn run(
    env: &Hermetic,
    root: &Path,
    args: &[&str],
    env_extra: &[(&str, &str)],
) -> Result<Outcome, TestError> {
    let mut command = Command::new(BIN);
    env.apply(&mut command);
    command
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .env("ACCELERATOR_LOG", "warn")
        .env_remove("ACCELERATOR_MIGRATE_FORCE")
        .env_remove("ACCELERATOR_MIGRATE_DECISIONS_FILE");
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

pub fn work_item(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"{id}\"\ntitle: {title}\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# {id}: {title}\n{body}"
    )
}

pub fn mark_all_migrations_applied(root: &Path) -> Result<(), TestError> {
    let ledger: String = MIGRATIONS_BEFORE_0007
        .iter()
        .chain(MIGRATIONS_FROM_0007.iter())
        .flat_map(|id| [*id, "\n"])
        .collect();
    write(root, ".accelerator/state/migrations-applied", &ledger)
}

pub fn mark_migrations_before_0006_applied(
    root: &Path,
) -> Result<(), TestError> {
    let ledger: String = MIGRATIONS_BEFORE_0007[..5]
        .iter()
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

pub const OWNED_NOTE: &str = "meta/research/codebase/2026-01-01-one.md";

/// A corpus on which 0006 rewrites [`OWNED_NOTE`] and records it in the run's
/// manifest before 0007 stalls on an ambiguous reference, so the stalled run
/// owns migrated output as well as its bookkeeping.
pub fn seed_output_then_ambiguous_reference(
    root: &Path,
) -> Result<(), TestError> {
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
    write(
        root,
        OWNED_NOTE,
        "---\ntype: codebase-research\nid: \"2026-01-01-one\"\n\
         title: \"One\"\ndate: \"2026-01-01T00:00:00+00:00\"\n\
         researcher: alice\ntags: []\nschema_version: 1\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\n\
         last_updated_by: alice\nrepository: \"r\"\n\
         revision: \"0123456789abcdef0123456789abcdef01234567\"\n\
         topic: \"One\"\n---\n\n# One\n",
    )?;
    mark_migrations_before_0006_applied(root)
}
