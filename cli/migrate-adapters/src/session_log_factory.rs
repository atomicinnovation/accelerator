//! One interactive migration's session log, bound to its own path.

use std::path::PathBuf;

use corpus::Clock;
use corpus::Outcome;
use corpus::Record;
use corpus::RecordStore;
use corpus::StoreError;
use migrate::ports::MigrationError;
use migrate::ports::SessionLog;
use migrate::ports::SessionLogFactory;

use crate::session_log::session_log_path;

/// The only session-log `schema_version` this port understands. A record
/// carrying any other value fails the read closed rather than being
/// silently reinterpreted or discarded.
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

pub struct FileSessionLog<'a> {
    records: &'a dyn RecordStore,
    clock: &'a dyn Clock,
    path: PathBuf,
}

impl FileSessionLog<'_> {
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

fn read_failure(error: StoreError) -> MigrationError {
    match error {
        StoreError::Io { detail, .. } | StoreError::Validation { detail } => {
            MigrationError::new(detail)
        }
        other => MigrationError::new(other.to_string()),
    }
}

impl SessionLog for FileSessionLog<'_> {
    fn records(&self) -> Result<Vec<Record>, MigrationError> {
        self.records
            .read_records(&self.path)
            .map_err(read_failure)?
            .into_iter()
            .map(|record| self.check_schema_version(record))
            .collect()
    }

    fn append(
        &self,
        key: &str,
        outcome: Outcome,
        proposed_value: &str,
        user_value: Option<&str>,
    ) -> Result<(), MigrationError> {
        let record = Record {
            transformation_key: key.to_owned(),
            schema_version: 1,
            outcome,
            proposed_value: proposed_value.to_owned(),
            user_value: user_value.map(str::to_owned),
            timestamp: self.clock.now_utc_iso(),
            extras: Vec::new(),
        };
        self.records
            .append_record(&self.path, &record)
            .map_err(|error| MigrationError::new(error.to_string()))
    }

    fn remove_by_key(&self, key: &str) -> Result<(), MigrationError> {
        self.records
            .remove_by_key(&self.path, key)
            .map_err(|error| MigrationError::new(error.to_string()))
    }
}

pub struct FileSessionLogFactory<'a> {
    root: PathBuf,
    records: &'a dyn RecordStore,
    clock: &'a dyn Clock,
}

impl<'a> FileSessionLogFactory<'a> {
    #[must_use]
    pub fn new(
        root: impl Into<PathBuf>,
        records: &'a dyn RecordStore,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            root: root.into(),
            records,
            clock,
        }
    }
}

impl SessionLogFactory for FileSessionLogFactory<'_> {
    fn for_migration(&self, id: &str) -> Box<dyn SessionLog + '_> {
        Box::new(FileSessionLog {
            records: self.records,
            clock: self.clock,
            path: session_log_path(&self.root, id),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::Clock;
    use corpus::FilenameTimestampFormat;
    use corpus::Outcome;
    use corpus::Record;
    use corpus::RecordStore;
    use corpus::StoreError;

    use super::FileSessionLogFactory;
    use migrate::ports::SessionLogFactory as _;

    type TestError = Box<dyn std::error::Error>;

    struct StubRecords {
        read: Result<Vec<Record>, StoreError>,
        appended: RefCell<Vec<(PathBuf, Record)>>,
    }

    impl StubRecords {
        fn reading(read: Result<Vec<Record>, StoreError>) -> Self {
            Self {
                read,
                appended: RefCell::new(Vec::new()),
            }
        }
    }

    impl RecordStore for StubRecords {
        fn append_record(
            &self,
            path: &Path,
            record: &Record,
        ) -> Result<(), StoreError> {
            self.appended
                .borrow_mut()
                .push((path.to_path_buf(), record.clone()));
            Ok(())
        }

        fn remove_by_key(
            &self,
            _path: &Path,
            _key: &str,
        ) -> Result<(), StoreError> {
            Ok(())
        }

        fn read_records(
            &self,
            _path: &Path,
        ) -> Result<Vec<Record>, StoreError> {
            self.read.clone()
        }
    }

    struct FixedClock;

    impl Clock for FixedClock {
        fn now_utc_iso(&self) -> String {
            "2026-07-19T00:00:00+00:00".to_owned()
        }

        fn filename_timestamp(
            &self,
            _format: FilenameTimestampFormat,
        ) -> String {
            String::new()
        }
    }

    fn record(schema_version: u32) -> Record {
        Record {
            transformation_key: "k1".to_owned(),
            schema_version,
            outcome: Outcome::Accepted,
            proposed_value: "v1".to_owned(),
            user_value: None,
            timestamp: "2026-07-19T00:00:00+00:00".to_owned(),
            extras: Vec::new(),
        }
    }

    fn read_error_text(error: StoreError) -> Option<String> {
        let records = StubRecords::reading(Err(error));
        let factory =
            FileSessionLogFactory::new("/repo", &records, &FixedClock);
        let log = factory.for_migration("0099");
        log.records().err().map(|error| error.to_string())
    }

    #[test]
    fn a_supported_schema_version_reads_back_fine() -> Result<(), TestError> {
        let records = StubRecords::reading(Ok(vec![record(1)]));
        let factory =
            FileSessionLogFactory::new("/repo", &records, &FixedClock);
        let log = factory.for_migration("0099");

        assert_eq!(log.records()?, vec![record(1)]);
        Ok(())
    }

    #[test]
    fn an_unsupported_schema_version_refuses_naming_the_discard_command() {
        let records = StubRecords::reading(Ok(vec![record(99)]));
        let factory =
            FileSessionLogFactory::new("/repo", &records, &FixedClock);
        let log = factory.for_migration("0099");

        assert_eq!(
            log.records().err().map(|error| error.to_string()),
            Some(
                "[resume] unknown schema_version 99 — supported: {1}.\n\
                 [resume] To discard the session and re-prompt, run:\n\
                 [resume]   rm /repo/.accelerator/state/migrations-0099-session.jsonl"
                    .to_owned()
            )
        );
    }

    #[test]
    fn an_io_failure_reads_as_its_bare_detail() {
        assert_eq!(
            read_error_text(StoreError::Io {
                path: "/repo/log".to_owned(),
                detail: "Is a directory (os error 21)".to_owned(),
            }),
            Some("Is a directory (os error 21)".to_owned())
        );
    }

    #[test]
    fn a_validation_failure_reads_as_its_bare_detail() {
        assert_eq!(
            read_error_text(StoreError::Validation {
                detail: "invalid record: record is not a JSON object"
                    .to_owned(),
            }),
            Some("invalid record: record is not a JSON object".to_owned())
        );
    }

    #[test]
    fn any_other_store_failure_reads_as_its_display_text() {
        assert_eq!(
            read_error_text(StoreError::LockTimeout {
                path: "/repo/log".to_owned(),
            }),
            Some("lock acquisition timed out on '/repo/log'".to_owned())
        );
    }

    #[test]
    fn an_append_is_stamped_by_the_clock_at_the_migrations_log(
    ) -> Result<(), TestError> {
        let records = StubRecords::reading(Ok(Vec::new()));
        let factory =
            FileSessionLogFactory::new("/repo", &records, &FixedClock);
        let log = factory.for_migration("0099");

        log.append("k1", Outcome::Accepted, "v1", None)?;

        assert_eq!(
            *records.appended.borrow(),
            vec![(
                PathBuf::from(
                    "/repo/.accelerator/state/migrations-0099-session.jsonl"
                ),
                record(1)
            )]
        );
        Ok(())
    }
}
