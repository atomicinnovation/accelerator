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

pub struct FileSessionLog {
    store: FileCorpusStore,
    path: PathBuf,
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
                corpus_adapters::jsonl::parse_record(line)
                    .map_err(|error| MigrationError::new(error.to_string()))
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
