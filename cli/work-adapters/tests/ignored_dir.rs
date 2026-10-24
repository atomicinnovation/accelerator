//! A state directory version control never sees, ready before anything is
//! written into it.

use std::cell::Cell;
use std::fs;
use std::path::Path;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use work_adapters::ignored_dir::prepare_ignored_dir;

type TestError = Box<dyn std::error::Error>;

struct CountingWrite {
    inner: FileCorpusStore,
    writes: Cell<usize>,
}

impl CountingWrite {
    fn new(root: &Path) -> Self {
        Self {
            inner: FileCorpusStore::new(root),
            writes: Cell::new(0),
        }
    }
}

impl AtomicWrite for CountingWrite {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.writes.set(self.writes.get() + 1);
        self.inner.write(path, bytes)
    }
}

#[test]
fn prepare_ignored_dir_writes_and_verifies_the_rule() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("state/rekey");

    prepare_ignored_dir(&dir, &FileCorpusStore::new(root.path()))?;

    assert_eq!(fs::read_to_string(dir.join(".gitignore"))?, "*\n");
    Ok(())
}

#[test]
fn an_existing_rule_is_left_untouched() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("state/rekey");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(".gitignore"), "*\n")?;
    let writer = CountingWrite::new(root.path());

    prepare_ignored_dir(&dir, &writer)?;

    assert_eq!(writer.writes.get(), 0);
    Ok(())
}

#[test]
fn concurrent_prepares_both_verify() -> Result<(), TestError> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("state/locks");

    std::thread::scope(|scope| -> Result<(), TestError> {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    prepare_ignored_dir(
                        &dir,
                        &FileCorpusStore::new(root.path()),
                    )
                })
            })
            .collect();
        for handle in handles {
            handle.join().map_err(|_| "a preparing thread panicked")??;
        }
        Ok(())
    })?;

    assert_eq!(fs::read_to_string(dir.join(".gitignore"))?, "*\n");
    Ok(())
}
