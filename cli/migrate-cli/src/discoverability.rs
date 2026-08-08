//! `migrate --discoverability-hook`: a `SessionStart` advisory when the
//! applied-migration ledger lags the compiled-in registry, reproducing
//! `hooks/migrate-discoverability.sh`.

use std::path::Path;
use std::path::PathBuf;

use migrate::registry::MigrationEntry;

fn is_accelerator_managed(root: &Path) -> bool {
    root.join(".accelerator").is_dir()
        || root.join(".claude/accelerator.md").is_file()
        || root.join("meta").is_dir()
}

/// The state file to read: the post-migration path when it exists, else the
/// legacy path — a deprecation-track shim, the only place this hook reads
/// the legacy location. Per-file, not per-directory, so a partial-recovery
/// repo (`.accelerator/` present but its own state file absent) still falls
/// through to the legacy path.
fn state_file(root: &Path) -> PathBuf {
    let current = root.join(".accelerator/state/migrations-applied");
    if current.is_file() {
        current
    } else {
        root.join("meta/.migrations-applied")
    }
}

fn highest_applied(state_file: &Path) -> Option<String> {
    let content = std::fs::read_to_string(state_file).ok()?;
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .max()
        .map(str::to_owned)
}

fn highest_available(entries: &[MigrationEntry]) -> Option<&'static str> {
    entries.iter().map(MigrationEntry::id).max()
}

/// The `SessionStart` envelope carrying the advisory, or `None` when there
/// is nothing to report: a non-Accelerator-managed repo, no registered
/// migrations, or a ledger already at or ahead of the highest registered
/// migration.
#[must_use]
pub fn run(root: &Path, entries: &[MigrationEntry]) -> Option<String> {
    if !is_accelerator_managed(root) {
        return None;
    }
    let highest_available = highest_available(entries)?;
    let state_file = state_file(root);
    let highest_applied = highest_applied(&state_file);
    let behind = highest_applied
        .as_ref()
        .is_none_or(|applied| applied.as_str() < highest_available);
    if !behind {
        return None;
    }
    let applied_label = highest_applied.as_deref().unwrap_or("(none)");
    let advisory = format!(
        "[accelerator] {} is behind the plugin (highest applied: \
         {applied_label}; highest available: {highest_available}). Run \
         /accelerator:migrate to bring it up to date.",
        state_file.display(),
    );
    Some(kernel::hooks::session_start("", Some(&advisory)))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::run;
    use migrate::registry::registry;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn silent_on_a_non_accelerator_repo() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        assert_eq!(run(dir.path(), &registry()), None);
        Ok(())
    }

    #[test]
    fn triggers_on_a_pre_migration_repo_with_accelerator_md(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join(".claude"))?;
        std::fs::write(dir.path().join(".claude/accelerator.md"), "")?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains("is behind the plugin"));
        Ok(())
    }

    #[test]
    fn triggers_on_a_pre_migration_repo_with_only_meta() -> Result<(), TestError>
    {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join("meta"))?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains("is behind the plugin"));
        Ok(())
    }

    #[test]
    fn reads_the_new_state_file_when_it_exists() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        std::fs::write(
            dir.path().join(".accelerator/state/migrations-applied"),
            "0001\n",
        )?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains(".accelerator/state/migrations-applied"));
        assert!(output.contains("is behind the plugin"));
        Ok(())
    }

    #[test]
    fn falls_back_to_the_legacy_state_file_when_accelerator_is_absent(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join("meta"))?;
        std::fs::write(dir.path().join("meta/.migrations-applied"), "0001\n")?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains("meta/.migrations-applied"));
        Ok(())
    }

    #[test]
    fn partial_recovery_falls_back_when_accelerator_has_no_state_file(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join(".accelerator"))?;
        std::fs::create_dir_all(dir.path().join("meta"))?;
        std::fs::write(
            dir.path().join("meta/.migrations-applied"),
            "0001\n0002\n",
        )?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains("meta/.migrations-applied"));
        assert!(output.contains("is behind the plugin"));
        Ok(())
    }

    #[test]
    fn silent_once_every_registered_migration_is_applied(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        let highest = registry()
            .iter()
            .map(migrate::registry::MigrationEntry::id)
            .max()
            .ok_or("registry is non-empty")?;
        std::fs::write(
            dir.path().join(".accelerator/state/migrations-applied"),
            format!("{highest}\n"),
        )?;
        assert_eq!(run(dir.path(), &registry()), None);
        Ok(())
    }

    #[test]
    fn ignores_blank_lines_in_the_state_file() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        let highest = registry()
            .iter()
            .map(migrate::registry::MigrationEntry::id)
            .max()
            .ok_or("registry is non-empty")?;
        std::fs::write(
            dir.path().join(".accelerator/state/migrations-applied"),
            format!("\n{highest}\n\n"),
        )?;
        assert_eq!(run(dir.path(), &registry()), None);
        Ok(())
    }

    #[test]
    fn the_envelope_is_a_bare_system_message_with_no_context(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        std::fs::create_dir_all(dir.path().join("meta"))?;
        let output = run(dir.path(), &registry()).ok_or("expected output")?;
        assert!(output.contains("\"systemMessage\""));
        assert!(output.contains("\"hookSpecificOutput\""));
        assert!(output.contains("\"additionalContext\":\"\""));
        Ok(())
    }
}
