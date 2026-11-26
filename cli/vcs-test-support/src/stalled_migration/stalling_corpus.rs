//! The corpus on which migrate's 0007 run stalls, and the invocation that
//! runs migrate over it the way every migrate test does.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use crate::hermetic::Hermetic;
use crate::Error;

const MIGRATION_0006: &str = "0006-canonicalise-work-item-id-and-author";

pub const MIGRATION_0007: &str = "0007-unify-meta-corpus-frontmatter";

/// Every migration migrate's registry holds, in the order it applies them.
pub const MIGRATIONS: [&str; 11] = [
    "0001-rename-tickets-to-work",
    "0002-rename-work-items-with-project-prefix",
    "0003-relocate-accelerator-state",
    "0004-restructure-meta-research-into-subject-subcategories",
    "0005-rename-work-item-type-to-kind",
    MIGRATION_0006,
    MIGRATION_0007,
    "0008-canonical-frontmatter-quoting",
    "0009-split-work-key-from-tracker-scope-key",
    "0010-strip-research-title-prefix",
    "0011-ignore-machine-local-sync-state",
];

pub const OWNED_NOTE: &str = "meta/research/codebase/2026-01-01-one.md";

const LEDGER: &str = ".accelerator/state/migrations-applied";

/// Runs `migrate` in `root` under `hermetic`, with no stdin and warnings
/// logged, so an assertion on the absence of a `WARN` line is never vacuous.
#[must_use]
pub fn migrate_command(
    migrate: &Path,
    root: &Path,
    hermetic: &Hermetic,
) -> Command {
    let mut command = Command::new(migrate);
    hermetic.apply(&mut command);
    command
        .current_dir(root)
        .stdin(Stdio::null())
        .env("ACCELERATOR_LOG", "warn")
        .env_remove("ACCELERATOR_FORCE")
        .env_remove("ACCELERATOR_MIGRATE_FORCE")
        .env_remove("ACCELERATOR_MIGRATE_DECISIONS_FILE");
    command
}

#[must_use]
pub fn work_item(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"{id}\"\ntitle: {title}\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# {id}: {title}\n{body}"
    )
}

/// The migrations [`MIGRATIONS`] applies before `migration`.
///
/// # Errors
///
/// When [`MIGRATIONS`] holds no `migration`.
pub fn migrations_before(
    migration: &str,
) -> Result<&'static [&'static str], Error> {
    MIGRATIONS
        .iter()
        .position(|id| *id == migration)
        .map(|end| &MIGRATIONS[..end])
        .ok_or_else(|| Error::message(format!("no migration {migration}")))
}

/// Records `migrations` in the ledger as already applied.
///
/// # Errors
///
/// When the ledger cannot be written.
pub fn mark_applied(root: &Path, migrations: &[&str]) -> Result<(), Error> {
    let ledger: String = migrations.iter().flat_map(|id| [*id, "\n"]).collect();
    write(root, LEDGER, &ledger)
}

/// A corpus on which 0006 rewrites [`OWNED_NOTE`] and records it in the run's
/// manifest before 0007 stalls on an ambiguous reference, so the stalled run
/// owns migrated output as well as its bookkeeping.
///
/// # Errors
///
/// When a file cannot be written.
pub fn seed_output_then_ambiguous_reference(root: &Path) -> Result<(), Error> {
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
    mark_applied(root, migrations_before(MIGRATION_0006)?)
}

fn write(root: &Path, relative: &str, content: &str) -> Result<(), Error> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}
