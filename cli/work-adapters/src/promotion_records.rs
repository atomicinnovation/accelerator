//! Where each draft's promotion record is kept: the pending-push file named
//! after its draft ID, beside any legacy markers.

use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use tracker::ExternalId;
use work::draft_id::DraftId;
use work::promotion::PromotionRecord;

use crate::sync::pending_push;
use crate::sync::pending_push::Marker;
use crate::sync::pending_push::UnreadableMarker;

/// A draft's record as read: absent, present, or a file that cannot be
/// understood as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredRecord {
    Absent,
    Unreadable(UnreadableMarker),
    Present(Box<PromotionRecord>),
}

impl StoredRecord {
    /// The key a present record holds, once the draft's issue exists.
    #[must_use]
    pub fn key(&self) -> Option<ExternalId> {
        match self {
            Self::Present(record) => record.stage.key().map(ExternalId::from),
            Self::Absent | Self::Unreadable(_) => None,
        }
    }
}

pub trait PromotionRecords {
    fn read(&self, draft: &DraftId) -> StoredRecord;

    /// # Errors
    /// [`StoreError`] when the record cannot be written.
    fn save(&self, record: &PromotionRecord) -> Result<(), StoreError>;

    /// Removing a record that is already gone succeeds.
    ///
    /// # Errors
    /// [`StoreError`] when the record exists but cannot be removed.
    fn remove(&self, draft: &DraftId) -> Result<(), StoreError>;

    /// Every promotion record, and every file named after a draft ID that
    /// cannot be read as one, each of which may stand for a create in
    /// flight.
    ///
    /// # Errors
    /// [`StoreError`] when the records cannot be listed.
    fn outstanding(
        &self,
    ) -> Result<Vec<Result<PromotionRecord, UnreadableMarker>>, StoreError>;
}

pub struct FilePromotionRecords<'a> {
    integrations_root: PathBuf,
    integration: String,
    writer: &'a dyn AtomicWrite,
}

impl<'a> FilePromotionRecords<'a> {
    #[must_use]
    pub fn new(
        integrations_root: &Path,
        integration: &str,
        writer: &'a dyn AtomicWrite,
    ) -> Self {
        Self {
            integrations_root: integrations_root.to_path_buf(),
            integration: integration.to_owned(),
            writer,
        }
    }

    #[must_use]
    pub fn path_of(&self, draft: &DraftId) -> PathBuf {
        pending_push::record_path(
            &self.integrations_root,
            &self.integration,
            draft,
        )
    }
}

fn io(path: &Path, detail: impl std::fmt::Display) -> StoreError {
    StoreError::Io {
        path: path.display().to_string(),
        detail: detail.to_string(),
    }
}

fn names_a_draft(path: &Path) -> bool {
    path.file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(DraftId::parse)
        .is_some()
}

impl PromotionRecords for FilePromotionRecords<'_> {
    fn read(&self, draft: &DraftId) -> StoredRecord {
        let path = self.path_of(draft);
        let unreadable = |detail: String| {
            StoredRecord::Unreadable(UnreadableMarker {
                path: path.clone(),
                detail,
            })
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return StoredRecord::Absent;
            }
            Err(error) => return unreadable(error.to_string()),
        };
        match pending_push::read_marker(Some(&content)) {
            Ok(Some(Marker::Promotion(record))) => {
                StoredRecord::Present(Box::new(record))
            }
            Ok(Some(Marker::Legacy(_))) => {
                unreadable("a legacy marker, not a promotion record".to_owned())
            }
            Ok(None) => StoredRecord::Absent,
            Err(error) => unreadable(error.to_string()),
        }
    }

    fn save(&self, record: &PromotionRecord) -> Result<(), StoreError> {
        let dir = pending_push::prepare_dir(
            &self.integrations_root,
            &self.integration,
        )
        .map_err(|error| io(&self.integrations_root, error))?;
        self.writer.write(
            &dir.join(format!("{}.json", record.draft_id.as_str())),
            pending_push::render_record(record).as_bytes(),
        )
    }

    fn remove(&self, draft: &DraftId) -> Result<(), StoreError> {
        let path = self.path_of(draft);
        match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(io(&path, error))
            }
            _ => Ok(()),
        }
    }

    fn outstanding(
        &self,
    ) -> Result<Vec<Result<PromotionRecord, UnreadableMarker>>, StoreError>
    {
        let markers = pending_push::outstanding(
            &self.integrations_root,
            &self.integration,
        )
        .map_err(|error| io(&self.integrations_root, error))?;
        Ok(markers
            .into_iter()
            .filter_map(|entry| match entry {
                Ok((_, Marker::Promotion(record))) => Some(Ok(record)),
                Err(unreadable) if names_a_draft(&unreadable.path) => {
                    Some(Err(unreadable))
                }
                Ok((_, Marker::Legacy(_))) | Err(_) => None,
            })
            .collect())
    }
}
