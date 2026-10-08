//! Realigning every integration's sync baseline after the corpus has been
//! re-rendered, so content-identical items are not reclassified as locally
//! modified.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;

use crate::sync::baseline;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::digest;

#[derive(Debug)]
pub enum RealignError {
    Listing(std::io::Error),
    Baseline(StoreError),
}

impl fmt::Display for RealignError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Listing(error) => error.fmt(formatter),
            Self::Baseline(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RealignError {}

/// Advances each entry that was `Synced` before the re-render to its file's
/// new digest, returning how many advanced.
///
/// A diverged entry is left untouched so its pending push survives.
/// `pre_migration` carries each re-rendered file's original content.
///
/// `writer_at` builds a writer rooted at a baseline's own directory, so an
/// integrations root outside the repository stays writable.
///
/// # Errors
///
/// [`RealignError::Listing`] when `integrations_root` cannot be listed, and
/// [`RealignError::Baseline`] when a baseline cannot be read or written.
pub fn realign_baselines(
    integrations_root: &Path,
    pre_migration: &[(PathBuf, String)],
    reader: &dyn FileReader,
    writer_at: &dyn Fn(&Path) -> Box<dyn AtomicWrite>,
) -> Result<usize, RealignError> {
    let mut by_id: HashMap<String, (&Path, &str)> = HashMap::new();
    for (path, original) in pre_migration {
        if let Some(id) = frontmatter_id(original) {
            by_id.insert(id, (path.as_path(), original.as_str()));
        }
    }

    let mut realigned = 0;
    for entry in
        fs::read_dir(integrations_root).map_err(RealignError::Listing)?
    {
        let entry = entry.map_err(RealignError::Listing)?;
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let integration = entry.file_name();
        let baseline_path =
            baseline::path(integrations_root, &integration.to_string_lossy());
        let writer = writer_at(&entry.path());
        let mut store =
            BaselineStore::new(baseline_path, reader, writer.as_ref());
        realigned += realign_one_baseline(&mut store, &by_id, reader)
            .map_err(RealignError::Baseline)?;
    }
    Ok(realigned)
}

fn frontmatter_id(content: &str) -> Option<String> {
    let (frontmatter, _) = digest::split_frontmatter_and_body(content).ok()?;
    let entries = corpus::frontmatter_validation::parse_entries(&frontmatter);
    let raw = corpus::frontmatter_validation::raw_value(&entries, "id")?;
    let id = corpus::frontmatter_validation::strip_surrounding_quote(raw);
    (!id.is_empty()).then(|| id.to_owned())
}

fn realign_one_baseline(
    store: &mut BaselineStore<'_>,
    by_id: &HashMap<String, (&Path, &str)>,
    reader: &dyn FileReader,
) -> Result<usize, StoreError> {
    let (baseline, _) = store.load()?;
    let mut realigned = 0;
    for (id, &(path, original)) in by_id {
        let Some(entry) = baseline.get(id) else {
            continue;
        };
        let Ok(pre_hash) = digest::local(original) else {
            continue;
        };
        if pre_hash != entry.local_hash {
            continue;
        }
        let Ok(Some(new_content)) = reader.read(path) else {
            continue;
        };
        let Ok(new_hash) = digest::local(&new_content) else {
            continue;
        };
        if new_hash == entry.local_hash {
            continue;
        }
        store.set(
            id,
            baseline::Entry {
                local_hash: new_hash,
                ..entry.clone()
            },
        )?;
        realigned += 1;
    }
    Ok(realigned)
}
