//! The record a differing adopt leaves, so the link it made stays
//! replaceable until a sync records the item's baseline.
//!
//! A recorded baseline confirms a link whatever a record says, so removing
//! a record once its item has a baseline entry is housekeeping, not a
//! safety step.

use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use serde_json::json;
use serde_json::Value;
use tracker::ExternalId;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;

use crate::sync::baseline::Baseline;
use crate::sync::create::canonical_external_key;
use crate::sync::fetch::LocalItem;
use crate::sync::files::SyncStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionRecord {
    pub previous_key: Option<ExternalId>,
    pub key: ExternalId,
    pub adopted_at: u64,
}

impl AdoptionRecord {
    /// Whether this record speaks for an item carrying `carried`: only
    /// while the item still carries a key the record names, so a fault
    /// between writing the record and the link leaves the link unconfirmed
    /// rather than silently confirmed.
    #[must_use]
    pub fn vouches_for(&self, carried: Option<&ExternalId>) -> bool {
        carried.is_some_and(|carried| {
            let carried = canonical_external_key(carried);
            canonical_external_key(&self.key) == carried
                || self.previous_key.as_ref().is_some_and(|previous| {
                    canonical_external_key(previous) == carried
                })
        })
    }
}

/// `None` when `id` is not one plain file-name component.
#[must_use]
pub fn path(
    integrations_dir: &Path,
    integration: &str,
    id: &str,
) -> Option<PathBuf> {
    let mut components = Path::new(id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) if name == id => Some(
            directory(integrations_dir, integration).join(format!("{id}.json")),
        ),
        _ => None,
    }
}

fn directory(integrations_dir: &Path, integration: &str) -> PathBuf {
    integrations_dir.join(integration).join("adopted")
}

#[must_use]
pub fn render(record: &AdoptionRecord) -> String {
    let rendered = json!({
        "previous_key": record.previous_key.as_ref().map(ExternalId::as_str),
        "key": record.key.as_str(),
        "adopted_at": record.adopted_at,
    });
    format!("{rendered}\n")
}

/// # Errors
///
/// A description of what is missing or malformed.
pub fn read(content: &str) -> Result<AdoptionRecord, String> {
    let value: Value =
        serde_json::from_str(content).map_err(|error| error.to_string())?;
    let key = value
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing 'key'".to_owned())?;
    let previous_key = match value.get("previous_key") {
        None | Some(Value::Null) => None,
        Some(Value::String(previous)) => Some(previous.clone()),
        Some(_) => return Err("malformed 'previous_key'".to_owned()),
    };
    let adopted_at = value
        .get("adopted_at")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing 'adopted_at'".to_owned())?;
    Ok(AdoptionRecord {
        previous_key: previous_key.map(ExternalId::new),
        key: ExternalId::new(key.to_owned()),
        adopted_at,
    })
}

/// Removes every record whose item now has a baseline entry, and returns
/// those it could not remove.
///
/// Every path that records a baseline runs this, so confirmation is
/// monotonic: a later loss of the entry cannot reopen the link.
pub fn retire_confirmed(
    integrations_dir: &Path,
    integration: &str,
    store: &dyn SyncStore,
    baseline: &Baseline,
) -> Vec<UnretiredRecord> {
    let dir = directory(integrations_dir, integration);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(std::ffi::OsStr::to_str) == Some("json")
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let id = path.file_stem()?.to_str()?.to_owned();
            baseline.get(&id)?;
            store.remove(&path).err().map(|error| UnretiredRecord {
                id,
                path,
                reason: error.to_string(),
            })
        })
        .collect()
}

/// A record whose item has a baseline entry, which could not be removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnretiredRecord {
    pub id: String,
    pub path: PathBuf,
    pub reason: String,
}

/// The adoption records of one integration, read as a create may rely on
/// them: one version control tracks, or whose tracking cannot be told, is
/// distrusted and ignored, so the link counts as confirmed.
pub struct AdoptionRecords<'a> {
    integrations_dir: &'a Path,
    integration: &'a str,
    tracking: &'a dyn RepositoryTracking,
}

impl<'a> AdoptionRecords<'a> {
    #[must_use]
    pub const fn new(
        integrations_dir: &'a Path,
        integration: &'a str,
        tracking: &'a dyn RepositoryTracking,
    ) -> Self {
        Self {
            integrations_dir,
            integration,
            tracking,
        }
    }

    #[must_use]
    pub fn path_of(&self, id: &str) -> Option<PathBuf> {
        path(self.integrations_dir, self.integration, id)
    }

    /// `item`'s record when it is trusted and vouches for the item's link.
    #[must_use]
    pub fn trusted(&self, item: &LocalItem) -> Option<AdoptionRecord> {
        let path = self.path_of(&item.id)?;
        let content = std::fs::read_to_string(&path).ok()?;
        if self.tracking.file_tracking(&path) != FileTracking::Untracked {
            return None;
        }
        read(&content)
            .ok()
            .filter(|record| record.vouches_for(item.external_id.as_ref()))
    }

    /// Rewrites whichever of `id`'s record keys named `old` to `new`, so a
    /// followed key keeps the record that vouches for it.
    ///
    /// # Errors
    ///
    /// The cause when the rewritten record cannot be written.
    pub fn follow(
        &self,
        store: &dyn SyncStore,
        id: &str,
        old: &ExternalId,
        new: &ExternalId,
    ) -> Result<(), String> {
        let Some((path, record)) = self.read(id) else {
            return Ok(());
        };
        let follows = |key: &ExternalId| {
            if canonical_external_key(key) == canonical_external_key(old) {
                new.clone()
            } else {
                key.clone()
            }
        };
        let followed = AdoptionRecord {
            previous_key: record.previous_key.as_ref().map(follows),
            key: follows(&record.key),
            ..record
        };
        if followed == record {
            return Ok(());
        }
        store
            .write(&path, render(&followed).as_bytes())
            .map_err(|error| error.to_string())
    }

    /// Moves `old_id`'s record to `new_id`, as a retirement renames the
    /// item it vouches for.
    ///
    /// # Errors
    ///
    /// The cause when the record cannot be written under its new id or
    /// removed from its old one.
    pub fn rename(
        &self,
        store: &dyn SyncStore,
        old_id: &str,
        new_id: &str,
        followed: (&ExternalId, &ExternalId),
    ) -> Result<(), String> {
        let (Some((old_path, record)), Some(new_path)) =
            (self.read(old_id), self.path_of(new_id))
        else {
            return Ok(());
        };
        let (old, new) = followed;
        let renamed = AdoptionRecord {
            key: if canonical_external_key(&record.key)
                == canonical_external_key(old)
            {
                new.clone()
            } else {
                record.key.clone()
            },
            ..record
        };
        store
            .write(&new_path, render(&renamed).as_bytes())
            .and_then(|()| store.remove(&old_path))
            .map_err(|error| error.to_string())
    }

    fn read(&self, id: &str) -> Option<(PathBuf, AdoptionRecord)> {
        let path = self.path_of(id)?;
        let record = read(&std::fs::read_to_string(&path).ok()?).ok()?;
        Some((path, record))
    }

    /// The ids among `items` whose link is unconfirmed: a trusted record
    /// vouches for it and no baseline entry has confirmed it.
    #[must_use]
    pub fn unconfirmed(
        &self,
        items: &[LocalItem],
        baseline: &Baseline,
    ) -> BTreeSet<String> {
        items
            .iter()
            .filter(|item| baseline.get(&item.id).is_none())
            .filter(|item| self.trusted(item).is_some())
            .map(|item| item.id.clone())
            .collect()
    }

    /// As [`retire_confirmed`], for this integration's records.
    pub fn retire_confirmed(
        &self,
        store: &dyn SyncStore,
        baseline: &Baseline,
    ) -> Vec<UnretiredRecord> {
        retire_confirmed(
            self.integrations_dir,
            self.integration,
            store,
            baseline,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use tracker::ExternalId;

    use super::read;
    use super::render;
    use super::AdoptionRecord;

    fn key(raw: &str) -> ExternalId {
        ExternalId::new(raw.to_owned())
    }

    fn record() -> AdoptionRecord {
        AdoptionRecord {
            previous_key: Some(key("ENG-1")),
            key: key("ENG-7"),
            adopted_at: 1_700_000_000,
        }
    }

    #[test]
    fn a_record_vouches_only_for_an_item_carrying_a_key_it_names() {
        for (carried, vouches) in [
            (None, false),
            (Some(key("eng-7")), true),
            (Some(key("ENG-1")), true),
            (Some(key("ENG-9")), false),
        ] {
            assert_eq!(
                record().vouches_for(carried.as_ref()),
                vouches,
                "{carried:?}"
            );
        }
    }

    #[test]
    fn a_record_reads_back_what_was_rendered() {
        for record in [
            record(),
            AdoptionRecord {
                previous_key: None,
                ..record()
            },
        ] {
            assert_eq!(read(&render(&record)).expect("a record"), record);
        }
    }

    #[test]
    fn a_record_without_a_key_is_malformed() {
        assert!(read("{\"adopted_at\": 1}").is_err());
    }
}
