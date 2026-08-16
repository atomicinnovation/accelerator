//! The baseline's persistence wrapper: read-modify-write through injected
//! ports on both sides.

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

    fn write_document(&self, baseline: &Baseline) -> Result<(), StoreError> {
        self.writer.write(&self.path, baseline.render().as_bytes())
    }

    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn set(&mut self, id: &str, entry: Entry) -> Result<(), StoreError> {
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

    /// Blanks the named items' `local_hash` and advances the timestamp as
    /// one operation: the ordering is load-bearing, and a two-call API
    /// could be called in the wrong order or half-called. Both mutations
    /// reach one in-memory document before the single write, so a failure
    /// loses neither in isolation.
    ///
    /// # Errors
    ///
    /// [`StoreError`] on either the read or the write.
    pub fn finalise_run(
        &mut self,
        blank: &[&str],
        run_start_epoch: u64,
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
        baseline.set_timestamp(run_start_epoch);
        self.write_document(&baseline)
    }
}
