//! Where the records of retirements in progress are kept, so an interrupted
//! one is finished by the next sync.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use serde_json::json;
use serde_json::Value;
use work::retirement::RetirementRecord;

pub trait RetirementRecords {
    /// # Errors
    /// [`StoreError`] when the record cannot be written.
    fn save(&self, record: &RetirementRecord) -> Result<(), StoreError>;

    /// # Errors
    /// [`StoreError`] when the records cannot be listed, or one cannot be
    /// read or understood.
    fn outstanding(&self) -> Result<Vec<RetirementRecord>, StoreError>;

    /// Removing a record that is already gone succeeds.
    ///
    /// # Errors
    /// [`StoreError`] when the record exists but cannot be removed.
    fn remove(&self, record: &RetirementRecord) -> Result<(), StoreError>;
}

const SCHEMA: u64 = 1;
const IGNORE_EVERYTHING: &str = "*\n";

/// One JSON file per record under `<state>/retirement-records/`, in a
/// directory ignored by version control before the first record lands.
pub struct FileRetirementRecords<'a> {
    dir: PathBuf,
    writer: &'a dyn AtomicWrite,
}

impl<'a> FileRetirementRecords<'a> {
    #[must_use]
    pub fn new(state_dir: &Path, writer: &'a dyn AtomicWrite) -> Self {
        Self {
            dir: state_dir.join("retirement-records"),
            writer,
        }
    }

    fn path_of(
        &self,
        record: &RetirementRecord,
    ) -> Result<PathBuf, StoreError> {
        let name = format!("{}--{}.json", record.old, record.new);
        let mut components = Path::new(&name).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(self.dir.join(name)),
            _ => Err(StoreError::Validation {
                detail: format!("'{name}' is not a record file name"),
            }),
        }
    }

    fn prepare(&self) -> Result<(), StoreError> {
        let io = |path: &Path, error: &std::io::Error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        };
        std::fs::create_dir_all(&self.dir)
            .map_err(|error| io(&self.dir, &error))?;
        let ignore = self.dir.join(".gitignore");
        std::fs::write(&ignore, IGNORE_EVERYTHING)
            .map_err(|error| io(&ignore, &error))?;
        let verified = std::fs::read_to_string(&ignore)
            .map_err(|error| io(&ignore, &error))?;
        if verified == IGNORE_EVERYTHING {
            Ok(())
        } else {
            Err(StoreError::Io {
                path: ignore.display().to_string(),
                detail: "the ignore rule could not be verified".to_owned(),
            })
        }
    }
}

fn encode(record: &RetirementRecord) -> String {
    json!({
        "schema": SCHEMA,
        "old": record.old,
        "new": record.new,
        "new_external_id": record.new_external_id,
        "recovery_dir": record.recovery_dir.display().to_string(),
    })
    .to_string()
        + "\n"
}

fn decode(path: &Path, raw: &str) -> Result<RetirementRecord, StoreError> {
    let malformed = |why: &str| StoreError::Validation {
        detail: format!("{}: {why}", path.display()),
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| malformed(&error.to_string()))?;
    if value.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
        return Err(malformed("unsupported schema"));
    }
    let text = |field: &str| {
        value
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| malformed(&format!("missing '{field}'")))
    };
    Ok(RetirementRecord {
        old: text("old")?,
        new: text("new")?,
        new_external_id: value
            .get("new_external_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        recovery_dir: PathBuf::from(text("recovery_dir")?),
    })
}

impl RetirementRecords for FileRetirementRecords<'_> {
    fn save(&self, record: &RetirementRecord) -> Result<(), StoreError> {
        self.prepare()?;
        self.writer
            .write(&self.path_of(record)?, encode(record).as_bytes())
    }

    fn outstanding(&self) -> Result<Vec<RetirementRecord>, StoreError> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => {
                return Err(StoreError::Io {
                    path: self.dir.display().to_string(),
                    detail: error.to_string(),
                });
            }
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();
        paths
            .iter()
            .map(|path| {
                let raw = std::fs::read_to_string(path).map_err(|error| {
                    StoreError::Io {
                        path: path.display().to_string(),
                        detail: error.to_string(),
                    }
                })?;
                decode(path, &raw)
            })
            .collect()
    }

    fn remove(&self, record: &RetirementRecord) -> Result<(), StoreError> {
        let path = self.path_of(record)?;
        match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(StoreError::Io {
                    path: path.display().to_string(),
                    detail: error.to_string(),
                })
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use corpus::store::AtomicWrite;
    use corpus::StoreError;
    use work::retirement::Retirement;
    use work::retirement::RetirementRecord;

    use super::FileRetirementRecords;
    use super::RetirementRecords as _;

    struct PlainWrite;

    impl AtomicWrite for PlainWrite {
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            std::fs::write(path, bytes).map_err(|error| StoreError::Io {
                path: path.display().to_string(),
                detail: error.to_string(),
            })
        }
    }

    fn record() -> RetirementRecord {
        RetirementRecord::of(&Retirement {
            old_id: "PP-760",
            new_id: "ENG-42",
            new_external_id: Some("ENG-42"),
        })
    }

    #[test]
    fn a_saved_record_is_outstanding_until_removed() {
        let state = tempfile::tempdir().unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);

        records.save(&record()).unwrap();
        assert_eq!(records.outstanding().unwrap(), vec![record()]);

        records.remove(&record()).unwrap();
        assert!(records.outstanding().unwrap().is_empty());
    }

    #[test]
    fn a_record_whose_ids_would_name_a_path_is_refused() {
        let state = tempfile::tempdir().unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);
        let escaping = RetirementRecord::of(&Retirement {
            old_id: "PP-760",
            new_id: "../../escaped",
            new_external_id: Some("../../escaped"),
        });

        assert!(matches!(
            records.save(&escaping),
            Err(StoreError::Validation { .. })
        ));
        assert!(matches!(
            records.remove(&escaping),
            Err(StoreError::Validation { .. })
        ));
        assert!(!state.path().join("escaped.json").exists());
    }

    #[test]
    fn the_record_file_carries_its_schema_and_is_named_by_both_ids() {
        let state = tempfile::tempdir().unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);

        records.save(&record()).unwrap();

        let raw = std::fs::read_to_string(
            state.path().join("retirement-records/PP-760--ENG-42.json"),
        )
        .unwrap();
        assert!(raw.contains("\"schema\":1"), "{raw}");
    }

    #[test]
    fn the_records_directory_is_ignored_before_the_first_record() {
        let state = tempfile::tempdir().unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);

        records.save(&record()).unwrap();

        assert_eq!(
            std::fs::read_to_string(
                state.path().join("retirement-records/.gitignore")
            )
            .unwrap(),
            "*\n"
        );
    }

    #[test]
    fn no_records_directory_means_nothing_outstanding() {
        let state = tempfile::tempdir().unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);

        assert!(records.outstanding().unwrap().is_empty());
        records.remove(&record()).unwrap();
    }

    #[test]
    fn a_malformed_record_is_reported_rather_than_skipped() {
        let state = tempfile::tempdir().unwrap();
        let dir = state.path().join("retirement-records");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("PP-1--ENG-1.json"), "{").unwrap();
        let records = FileRetirementRecords::new(state.path(), &PlainWrite);

        assert!(matches!(
            records.outstanding(),
            Err(StoreError::Validation { .. })
        ));
    }
}
