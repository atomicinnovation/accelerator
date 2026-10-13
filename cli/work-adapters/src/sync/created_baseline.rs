//! The sync baseline entry a create records for the issue it made, so the
//! next sync classifies the item against what the create left.

use corpus::store::StoreError;
use tracker::RemoteTimestamp;
use work::promotion::IntendedBaseline;
use work::promotion::RemoteHash;
use work::sync::LegacyHashes;

use crate::sync::baseline::Entry;
use crate::sync::baseline::SyncStamps;
use crate::sync::baseline_store::BaselineStore;

/// Writes `item_id`'s entry from `intended`, making no tracker call.
///
/// # Errors
///
/// [`StoreError`] when the baseline cannot be read or written.
pub fn record_created_baseline(
    item_id: &str,
    intended: &IntendedBaseline,
    baseline: &BaselineStore<'_>,
    synced_at: u64,
) -> Result<(), StoreError> {
    let (remote_updated_at, remote_hash) = match &intended.remote_hash {
        RemoteHash::Known(read_back) => {
            (read_back.updated.clone(), Some(read_back.hash.clone()))
        }
        RemoteHash::Unknown => (RemoteTimestamp::NotRead, None),
    };
    baseline.set(
        item_id,
        Entry::Legacy {
            stamps: SyncStamps {
                remote_updated_at,
                local_synced_at: synced_at,
            },
            legacy: LegacyHashes {
                local: Some(intended.local_hash.clone()),
                remote: remote_hash,
            },
        },
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;

    use corpus::scan::FileReader;
    use corpus::store::AtomicWrite;
    use corpus::store::StoreError;
    use tracker::RemoteTimestamp;
    use work::promotion::IntendedBaseline;
    use work::promotion::ReadBack;
    use work::promotion::RemoteHash;
    use work::sync::LegacyHashes;

    use super::record_created_baseline;
    use crate::sync::baseline_store::BaselineStore;

    struct Disk;

    impl FileReader for Disk {
        fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
            Ok(std::fs::read_to_string(path).ok())
        }
    }

    impl AtomicWrite for Disk {
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            std::fs::write(path, bytes).map_err(|error| StoreError::Io {
                path: path.display().to_string(),
                detail: error.to_string(),
            })
        }
    }

    fn recorded(remote_hash: RemoteHash) -> crate::sync::baseline::Entry {
        let dir = tempfile::tempdir().expect("tempdir");
        let store =
            BaselineStore::new(dir.path().join("last-sync.json"), &Disk, &Disk);
        record_created_baseline(
            "PP-900",
            &IntendedBaseline {
                remote_hash,
                local_hash: "local".to_owned(),
            },
            &store,
            42,
        )
        .expect("recorded");
        let (baseline, _) = store.load().expect("loads");
        baseline.get("PP-900").cloned().expect("an entry")
    }

    #[test]
    fn a_known_remote_records_its_hash_and_stamp() {
        let entry = recorded(RemoteHash::Known(ReadBack {
            hash: "remote".to_owned(),
            updated: RemoteTimestamp::Reported("2026-09-28".to_owned()),
        }));

        assert_eq!(
            entry.legacy(),
            Some(&LegacyHashes {
                local: Some("local".to_owned()),
                remote: Some("remote".to_owned()),
            })
        );
        let stamps = entry.stamps().expect("stamps");
        assert_eq!(
            stamps.remote_updated_at,
            RemoteTimestamp::Reported("2026-09-28".to_owned())
        );
        assert_eq!(stamps.local_synced_at, 42);
    }

    #[test]
    fn an_unknown_remote_records_no_hash_so_it_reads_as_changed() {
        let entry = recorded(RemoteHash::Unknown);

        assert_eq!(
            entry.legacy().and_then(|legacy| legacy.remote.clone()),
            None
        );
        assert_eq!(
            entry.stamps().map(|stamps| &stamps.remote_updated_at),
            Some(&RemoteTimestamp::NotRead)
        );
    }
}
