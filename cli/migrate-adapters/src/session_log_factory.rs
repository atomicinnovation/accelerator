//! One interactive migration's session log, bound to its own path.

use std::path::PathBuf;

use corpus::Clock as _;
use corpus::Outcome;
use corpus::Record;
use corpus::RecordStore as _;
use corpus_adapters::metadata::SystemClock;
use corpus_adapters::FileCorpusStore;
use migrate::ports::MigrationError;
use migrate::ports::SessionLog;
use migrate::ports::SessionLogFactory;
use time::UtcOffset;

use crate::session_log::session_log_path;

/// The only session-log `schema_version` this port understands. A record
/// carrying any other value fails the read closed rather than being
/// silently reinterpreted or discarded.
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

pub struct FileSessionLog {
    store: FileCorpusStore,
    path: PathBuf,
}

impl FileSessionLog {
    fn check_schema_version(
        &self,
        record: Record,
    ) -> Result<Record, MigrationError> {
        if record.schema_version == SUPPORTED_SCHEMA_VERSION {
            return Ok(record);
        }
        Err(MigrationError::new(format!(
            "[resume] unknown schema_version {} — supported: {{{}}}.\n\
             [resume] To discard the session and re-prompt, run:\n\
             [resume]   rm {}",
            record.schema_version,
            SUPPORTED_SCHEMA_VERSION,
            self.path.display()
        )))
    }
}

impl SessionLog for FileSessionLog {
    fn records(&self) -> Result<Vec<Record>, MigrationError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read(&self.path)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        let text = String::from_utf8(bytes)
            .map_err(|error| MigrationError::new(error.to_string()))?;
        text.lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let record = corpus_adapters::jsonl::parse_record(line)
                    .map_err(|error| MigrationError::new(error.to_string()))?;
                self.check_schema_version(record)
            })
            .collect()
    }

    fn append(
        &self,
        key: &str,
        outcome: Outcome,
        proposed_value: &str,
        user_value: Option<&str>,
    ) -> Result<(), MigrationError> {
        // `now_utc_iso` never reads the clock's offset, so pinning it to UTC
        // here avoids `SystemClock::try_new`'s `date +%z` subprocess — this
        // crate reads in-process only.
        let timestamp = SystemClock::with_offset(UtcOffset::UTC).now_utc_iso();
        let record = Record {
            transformation_key: key.to_owned(),
            schema_version: 1,
            outcome,
            proposed_value: proposed_value.to_owned(),
            user_value: user_value.map(str::to_owned),
            timestamp,
            extras: Vec::new(),
        };
        self.store
            .append_record(&self.path, &record)
            .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn remove_by_key(&self, key: &str) -> Result<(), MigrationError> {
        self.store
            .remove_by_key(&self.path, key)
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

pub struct FileSessionLogFactory {
    root: PathBuf,
}

impl FileSessionLogFactory {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl SessionLogFactory for FileSessionLogFactory {
    fn for_migration(&self, id: &str) -> Box<dyn SessionLog> {
        Box::new(FileSessionLog {
            store: FileCorpusStore::new(&self.root),
            path: session_log_path(&self.root, id),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::FileSessionLogFactory;
    use migrate::ports::SessionLogFactory as _;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_supported_schema_version_reads_back_fine() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        fs::write(
            dir.path()
                .join(".accelerator/state/migrations-0099-session.jsonl"),
            "{\"transformation_key\":\"k1\",\"schema_version\":1,\
             \"outcome\":\"accepted\",\"proposed_value\":\"v1\",\
             \"timestamp\":\"2026-07-19T00:00:00+00:00\"}\n",
        )?;
        let log = FileSessionLogFactory::new(dir.path()).for_migration("0099");

        let records = log.records()?;

        assert_eq!(records.len(), 1);
        Ok(())
    }

    #[test]
    fn an_unsupported_schema_version_refuses_naming_the_discard_command(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        fs::create_dir_all(dir.path().join(".accelerator/state"))?;
        let path = dir
            .path()
            .join(".accelerator/state/migrations-0099-session.jsonl");
        fs::write(
            &path,
            "{\"transformation_key\":\"k1\",\"schema_version\":99,\
             \"outcome\":\"accepted\",\"proposed_value\":\"v1\",\
             \"timestamp\":\"2026-07-19T00:00:00+00:00\"}\n",
        )?;
        let log = FileSessionLogFactory::new(dir.path()).for_migration("0099");

        let result = log.records();

        assert_eq!(
            result.err().map(|error| error.to_string()),
            Some(format!(
                "[resume] unknown schema_version 99 — supported: {{1}}.\n\
                 [resume] To discard the session and re-prompt, run:\n\
                 [resume]   rm {}",
                path.display()
            ))
        );
        Ok(())
    }

    #[test]
    fn no_session_log_file_is_an_empty_read() -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let log = FileSessionLogFactory::new(dir.path()).for_migration("0099");

        assert_eq!(log.records()?.len(), 0);
        Ok(())
    }
}
