//! The baseline's persistence wrapper: read-modify-write through injected
//! ports on both sides.

use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;

use crate::sync::baseline::Baseline;
use crate::sync::baseline::Degradation;
use crate::sync::baseline::Entry;

/// Reads and writes one baseline document.
///
/// Every mutation re-reads before it renders, rather than holding one
/// in-memory document for a whole run, which would widen the lost-update
/// window from a single write to the entire run.
///
/// The read side is injected as well as the write side: with a write-only
/// seam, a spy `AtomicWrite` would see the writes while reads still came
/// from disk, so successive `set` calls would each start from the pre-run
/// document.
pub struct BaselineStore<'a> {
    path: PathBuf,
    reader: &'a dyn FileReader,
    writer: &'a dyn AtomicWrite,
}

impl<'a> BaselineStore<'a> {
    #[must_use]
    pub const fn new(
        path: PathBuf,
        reader: &'a dyn FileReader,
        writer: &'a dyn AtomicWrite,
    ) -> Self {
        Self {
            path,
            reader,
            writer,
        }
    }

    /// Reads the document, reporting any degradation rather than hiding it.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the underlying read fails for a reason other
    /// than "the file does not exist".
    pub fn load(&self) -> Result<(Baseline, Degradation), StoreError> {
        let content =
            self.reader
                .read(&self.path)
                .map_err(|error| StoreError::Io {
                    path: self.path.display().to_string(),
                    detail: error.to_string(),
                })?;
        Ok(Baseline::read(content.as_deref()))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_document(&self, baseline: &Baseline) -> Result<(), StoreError> {
        self.writer.write(&self.path, baseline.render().as_bytes())
    }

    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn set(&self, id: &str, entry: Entry) -> Result<(), StoreError> {
        let (mut baseline, _) = self.load()?;
        baseline.set(id, entry);
        self.write_document(&baseline)
    }

    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn remove(&mut self, id: &str) -> Result<(), StoreError> {
        let (mut baseline, _) = self.load()?;
        baseline.remove(id);
        self.write_document(&baseline)
    }

    /// Moves `old`'s entry to `new` in one write, leaving every other
    /// entry as the re-read document has it; an absent entry writes nothing.
    ///
    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn rename(&self, old: &str, new: &str) -> Result<(), StoreError> {
        let (mut baseline, _) = self.load()?;
        let Some(entry) = baseline.get(old).cloned() else {
            return Ok(());
        };
        baseline.remove(old);
        baseline.set(new, entry);
        self.write_document(&baseline)
    }

    /// Forgets the named items' local digests, advances the reconciled items'
    /// watermarks, and — for a whole-corpus run — advances the document
    /// timestamp, as one operation: the ordering is load-bearing, and a
    /// multi-call API could be called in the wrong order or half-called. Every
    /// mutation reaches one in-memory document before the single write, so a
    /// failure loses none in isolation.
    ///
    /// `advance` names the items whose watermark moves to `run_start_epoch` —
    /// exactly those that reached a definitive reconciled outcome, so an
    /// unreconciled or indeterminate item keeps the watermark of the last run
    /// that did reconcile it. `advance_document` is the whole-corpus signal:
    /// only a run over the full corpus advances the document-level fallback
    /// watermark, so a targeted run leaves it where the last full sync set it.
    ///
    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn finalise_run(
        &mut self,
        blank: &[&str],
        advance: &[&str],
        run_start_epoch: u64,
        advance_document: bool,
    ) -> Result<(), StoreError> {
        let (mut baseline, _) = self.load()?;
        for id in blank {
            if let Some(entry) = baseline.get(id).cloned() {
                baseline.set(id, entry.without_local_digests());
            }
        }
        for id in advance {
            baseline.advance_watermark(id, run_start_epoch);
        }
        if advance_document {
            baseline.set_timestamp(run_start_epoch);
        }
        self.write_document(&baseline)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use corpus::scan::FileReader;
    use corpus::store::AtomicWrite;
    use corpus::store::StoreError;

    use tracker::RemoteTimestamp;
    use work::sync::LegacyHashes;
    use work::sync::LocalFingerprints;
    use work::sync::RemoteFingerprints;
    use work::sync::SplitFingerprints;

    use super::BaselineStore;
    use crate::sync::baseline::Entry;
    use crate::sync::baseline::SyncStamps;

    #[derive(Default)]
    struct MemoryFile {
        files: RefCell<BTreeMap<PathBuf, String>>,
        writes: RefCell<usize>,
    }

    impl FileReader for MemoryFile {
        fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
            Ok(self.files.borrow().get(path).cloned())
        }
    }

    impl AtomicWrite for MemoryFile {
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            *self.writes.borrow_mut() += 1;
            self.files.borrow_mut().insert(
                path.to_path_buf(),
                String::from_utf8_lossy(bytes).into_owned(),
            );
            Ok(())
        }
    }

    fn entry(local_hash: &str) -> Entry {
        Entry::Legacy {
            stamps: SyncStamps {
                remote_updated_at: RemoteTimestamp::NotRead,
                local_synced_at: 0,
            },
            legacy: LegacyHashes {
                local: Some(local_hash.to_owned()),
                remote: None,
            },
        }
    }

    #[test]
    fn renaming_an_entry_moves_it_in_one_write() -> Result<(), StoreError> {
        let file = MemoryFile::default();
        let store =
            BaselineStore::new(PathBuf::from("last-sync.json"), &file, &file);
        store.set("draft-k7mq3x", entry("moved"))?;
        store.set("0001", entry("kept"))?;
        *file.writes.borrow_mut() = 0;

        store.rename("draft-k7mq3x", "PP-900")?;

        let (baseline, _) = store.load()?;
        assert_eq!(*file.writes.borrow(), 1);
        assert_eq!(baseline.get("draft-k7mq3x"), None);
        assert_eq!(baseline.get("PP-900"), Some(&entry("moved")));
        assert_eq!(baseline.get("0001"), Some(&entry("kept")));
        Ok(())
    }

    #[test]
    fn finalise_run_blanks_both_split_local_hashes_and_the_legacy_local_hash(
    ) -> Result<(), StoreError> {
        let file = MemoryFile::default();
        let mut store =
            BaselineStore::new(PathBuf::from("last-sync.json"), &file, &file);
        let remote = RemoteFingerprints {
            body: "remote-body".to_owned(),
            fields: "remote-fields".to_owned(),
        };
        store.set(
            "0001",
            Entry::Split {
                stamps: SyncStamps {
                    remote_updated_at: RemoteTimestamp::NotRead,
                    local_synced_at: 0,
                },
                legacy: LegacyHashes {
                    local: Some("legacy-local".to_owned()),
                    remote: Some("legacy-remote".to_owned()),
                },
                split: SplitFingerprints {
                    local: Some(LocalFingerprints {
                        frontmatter: "frontmatter".to_owned(),
                        fields: "fields".to_owned(),
                        body: "body".to_owned(),
                    }),
                    remote: Some(remote.clone()),
                },
            },
        )?;

        store.finalise_run(&["0001"], &[], 1, true)?;

        let (baseline, _) = store.load()?;
        let entry = baseline.get("0001");
        assert_eq!(
            entry.and_then(Entry::legacy),
            Some(&LegacyHashes {
                local: None,
                remote: Some("legacy-remote".to_owned()),
            })
        );
        assert_eq!(
            entry.and_then(Entry::split),
            Some(&SplitFingerprints {
                local: None,
                remote: Some(remote),
            })
        );
        Ok(())
    }

    const UNKNOWN_RECIPE: &str =
        "{\"timestamp\":1,\"items\":{\"draft-k7mq3x\":\
        {\"digest_recipe\":2,\"local_hash\":\"l\"}}}\n";

    #[test]
    fn finalise_run_leaves_an_unknown_recipe_entry_unblanked(
    ) -> Result<(), StoreError> {
        let file = MemoryFile::default();
        file.files
            .borrow_mut()
            .insert(PathBuf::from("last-sync.json"), UNKNOWN_RECIPE.to_owned());
        let mut store =
            BaselineStore::new(PathBuf::from("last-sync.json"), &file, &file);

        store.finalise_run(&["draft-k7mq3x"], &["draft-k7mq3x"], 1, false)?;

        assert_eq!(
            file.files.borrow().get(Path::new("last-sync.json")),
            Some(&UNKNOWN_RECIPE.to_owned())
        );
        Ok(())
    }

    #[test]
    fn a_rename_carries_an_unknown_recipe_entry_to_the_new_id(
    ) -> Result<(), StoreError> {
        let file = MemoryFile::default();
        file.files
            .borrow_mut()
            .insert(PathBuf::from("last-sync.json"), UNKNOWN_RECIPE.to_owned());
        let store =
            BaselineStore::new(PathBuf::from("last-sync.json"), &file, &file);

        store.rename("draft-k7mq3x", "PP-900")?;

        assert_eq!(
            file.files.borrow().get(Path::new("last-sync.json")),
            Some(&UNKNOWN_RECIPE.replace("draft-k7mq3x", "PP-900"))
        );
        Ok(())
    }

    #[test]
    fn renaming_an_absent_entry_is_a_no_op() -> Result<(), StoreError> {
        let file = MemoryFile::default();
        let store =
            BaselineStore::new(PathBuf::from("last-sync.json"), &file, &file);
        store.set("0001", entry("kept"))?;
        *file.writes.borrow_mut() = 0;

        store.rename("draft-k7mq3x", "PP-900")?;

        assert_eq!(*file.writes.borrow(), 0);
        Ok(())
    }
}
