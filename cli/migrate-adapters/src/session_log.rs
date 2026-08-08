//! The interactive session log: `.accelerator/state/migrations-<id>-session.jsonl`.
//!
//! Ordinary appends/removes go straight through `corpus_adapters::FileCorpusStore`
//! (already a `corpus::RecordStore`) at that path — no wrapper needed. What
//! this module adds is the one capability that doesn't exist yet: the
//! bash-session-log-to-canonical-format cutover.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus_adapters::jsonl::compose_record;
use corpus_adapters::jsonl::parse_record;
use corpus_adapters::FileCorpusStore;
use migrate::ports::MigrationError;
use migrate::ports::SessionLogRewriter;

#[must_use]
pub fn session_log_path(root: &Path, migration_id: &str) -> PathBuf {
    root.join(format!(
        ".accelerator/state/migrations-{migration_id}-session.jsonl"
    ))
}

pub struct FileSessionLogRewriter {
    store: FileCorpusStore,
}

impl FileSessionLogRewriter {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            store: FileCorpusStore::new(root),
        }
    }
}

impl SessionLogRewriter for FileSessionLogRewriter {
    fn cutover(&self, path: &Path) -> Result<(), MigrationError> {
        if !path.exists() {
            return Ok(());
        }
        let bytes = fs::read(path)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        let text = String::from_utf8(bytes)
            .map_err(|error| MigrationError::new(error.to_string()))?;

        let mut recomposed = String::new();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            let record = parse_record(line)
                .map_err(|error| MigrationError::new(error.to_string()))?;
            let composed = compose_record(&record)
                .map_err(|error| MigrationError::new(error.to_string()))?;
            recomposed.push_str(&composed);
            recomposed.push('\n');
        }

        self.store
            .replace_locked(path, recomposed.as_bytes())
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::FileSessionLogRewriter;
    use migrate::ports::SessionLogRewriter as _;

    type TestError = Box<dyn std::error::Error>;

    const BASH_WRITTEN: &str = "{\"transformation_key\":\"link-A\",\"schema_version\":1,\"outcome\":\"edited\",\"proposed_value\":\"work-item:0001\",\"user_value\":\"work-item:0002\",\"timestamp\":\"2026-07-19T00:00:00+00:00\"}\n";

    #[test]
    fn a_bash_written_log_is_rewritten_to_canonical_form(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/log.jsonl");
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        fs::write(&path, BASH_WRITTEN)?;

        FileSessionLogRewriter::new(dir.path()).cutover(&path)?;

        assert_eq!(fs::read_to_string(&path)?, BASH_WRITTEN);
        Ok(())
    }

    #[test]
    fn an_already_canonical_log_is_byte_identical_after_cutover(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/log.jsonl");
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        fs::write(&path, BASH_WRITTEN)?;

        let rewriter = FileSessionLogRewriter::new(dir.path());
        rewriter.cutover(&path)?;
        let once = fs::read_to_string(&path)?;
        rewriter.cutover(&path)?;
        let twice = fs::read_to_string(&path)?;

        assert_eq!(once, twice);
        Ok(())
    }

    #[test]
    fn no_file_at_all_is_a_no_op() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/log.jsonl");

        FileSessionLogRewriter::new(dir.path()).cutover(&path)?;

        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn a_hand_truncated_record_refuses_and_leaves_the_file_unchanged(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/log.jsonl");
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        let truncated = "{\"transformation_key\":\"link-A\",\"schema_";
        fs::write(&path, truncated)?;

        let result = FileSessionLogRewriter::new(dir.path()).cutover(&path);

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path)?, truncated);
        Ok(())
    }

    #[test]
    fn an_empty_proposed_value_refuses_and_leaves_the_file_unchanged(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let path = dir.path().join(".accelerator/state/log.jsonl");
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        let invalid = "{\"transformation_key\":\"link-A\",\"schema_version\":1,\"outcome\":\"edited\",\"proposed_value\":\"\",\"timestamp\":\"2026-07-19T00:00:00+00:00\"}\n";
        fs::write(&path, invalid)?;

        let result = FileSessionLogRewriter::new(dir.path()).cutover(&path);

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path)?, invalid);
        Ok(())
    }
}
