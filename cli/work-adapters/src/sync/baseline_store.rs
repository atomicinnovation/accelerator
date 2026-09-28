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

    /// Blanks the named items' `local_hash`, advances the reconciled items'
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
                baseline.set(
                    id,
                    Entry {
                        local_hash: String::new(),
                        ..entry
                    },
                );
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

    use super::BaselineStore;
    use crate::sync::baseline::Entry;

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
        Entry {
            remote_updated_at: RemoteTimestamp::NotRead,
            remote_hash: String::new(),
            local_hash: local_hash.to_owned(),
            local_synced_at: 0,
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
