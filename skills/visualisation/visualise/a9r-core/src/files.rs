//! Config-file discovery and the legacy-layout guard.
//!
//! Ports `config_find_files` and `config_assert_no_legacy_layout` from
//! [`scripts/config-common.sh`](../../../../scripts/config-common.sh) (lines
//! 26-66).

use std::path::{Path, PathBuf};

use crate::ConfigError;

/// Config files that exist, in precedence order: team config first, local
/// config second. The caller relies on this order for last-writer-wins
/// (local overrides team). In migration mode, and only when neither new-layout
/// file exists, the legacy `.claude/accelerator.md` locations are added.
pub fn config_files(root: &Path, migration_mode: bool) -> Vec<PathBuf> {
    let team = root.join(".accelerator/config.md");
    let local = root.join(".accelerator/config.local.md");
    let mut files = Vec::new();
    if team.is_file() {
        files.push(team);
    }
    if local.is_file() {
        files.push(local);
    }
    if files.is_empty() && migration_mode {
        let legacy_team = root.join(".claude/accelerator.md");
        let legacy_local = root.join(".claude/accelerator.local.md");
        if legacy_team.is_file() {
            files.push(legacy_team);
        }
        if legacy_local.is_file() {
            files.push(legacy_local);
        }
    }
    files
}

/// Fail if the project uses the legacy `.claude/accelerator.md` layout (the
/// legacy file exists but the new `.accelerator/config.md` does not). Skipped
/// in migration mode.
pub fn assert_no_legacy_layout(root: &Path, migration_mode: bool) -> Result<(), ConfigError> {
    if migration_mode {
        return Ok(());
    }
    let team = root.join(".accelerator/config.md");
    let legacy_team = root.join(".claude/accelerator.md");
    if !team.is_file() && legacy_team.is_file() {
        return Err(ConfigError::LegacyLayout);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn team_then_local_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".accelerator/config.md"), "team");
        write(&root.join(".accelerator/config.local.md"), "local");
        let files = config_files(root, false);
        assert_eq!(
            files,
            vec![
                root.join(".accelerator/config.md"),
                root.join(".accelerator/config.local.md"),
            ],
        );
    }

    #[test]
    fn only_existing_files_are_listed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".accelerator/config.local.md"), "local");
        assert_eq!(
            config_files(root, false),
            vec![root.join(".accelerator/config.local.md")],
        );
    }

    #[test]
    fn legacy_fallback_only_in_migration_mode_and_only_when_new_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".claude/accelerator.md"), "legacy");
        // Not in migration mode → legacy ignored.
        assert!(config_files(root, false).is_empty());
        // Migration mode → legacy used.
        assert_eq!(
            config_files(root, true),
            vec![root.join(".claude/accelerator.md")],
        );
    }

    #[test]
    fn legacy_not_used_when_new_layout_present_even_in_migration_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".accelerator/config.md"), "team");
        write(&root.join(".claude/accelerator.md"), "legacy");
        assert_eq!(
            config_files(root, true),
            vec![root.join(".accelerator/config.md")],
        );
    }

    #[test]
    fn legacy_only_layout_fails_the_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".claude/accelerator.md"), "legacy");
        assert!(matches!(
            assert_no_legacy_layout(root, false),
            Err(ConfigError::LegacyLayout),
        ));
    }

    #[test]
    fn guard_passes_when_new_layout_present() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".accelerator/config.md"), "team");
        write(&root.join(".claude/accelerator.md"), "legacy");
        assert!(assert_no_legacy_layout(root, false).is_ok());
    }

    #[test]
    fn guard_passes_with_no_config_at_all() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(assert_no_legacy_layout(tmp.path(), false).is_ok());
    }

    #[test]
    fn guard_skipped_in_migration_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(&root.join(".claude/accelerator.md"), "legacy");
        assert!(assert_no_legacy_layout(root, true).is_ok());
    }
}
