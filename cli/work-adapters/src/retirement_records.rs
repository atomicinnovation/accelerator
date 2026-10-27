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

use crate::ignored_dir::prepare_ignored_dir;

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
        prepare_ignored_dir(&self.dir, self.writer)?;
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
