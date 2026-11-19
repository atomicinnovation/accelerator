//! Realigning every integration's sync baseline after a migration has
//! rewritten work items, so items that were in sync before it are not
//! reclassified as locally modified.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::body::BodyParser;
use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use tracker::BodyDialect;
use work::sync::LegacyHashes;
use work::sync::SplitFingerprints;

use crate::sync::baseline;
use crate::sync::baseline::Baseline;
use crate::sync::baseline_digester::BaselineDigester;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::digest;

#[derive(Debug)]
pub enum RealignError {
    Listing(std::io::Error),
    Baseline(StoreError),
    LeftRealigned {
        cause: StoreError,
        left_realigned_baselines: Vec<PathBuf>,
    },
}

impl fmt::Display for RealignError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Listing(error) => error.fmt(formatter),
            Self::Baseline(error) => error.fmt(formatter),
            Self::LeftRealigned {
                cause,
                left_realigned_baselines,
            } => {
                let named = left_realigned_baselines
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    formatter,
                    "{cause}; could not put back the baselines already \
                     realigned: {named}"
                )
            }
        }
    }
}

impl std::error::Error for RealignError {}

/// Advances each entry that was `Synced` before the migration to its file's
/// new digest, returning how many advanced.
///
/// A diverged entry is left untouched so its pending push survives.
/// `pre_migration` carries each rewritten file's original content.
///
/// `writer_at` builds a writer rooted at a baseline's own directory, so an
/// integrations root outside the repository stays writable, and
/// `body_rules` names the body dialect of the tracker an integration
/// directory belongs to. A directory with no known dialect is left
/// untouched, since digests taken under guessed rules would misclassify its
/// items on the next sync.
///
/// # Errors
///
/// [`RealignError::Listing`] when `integrations_root` cannot be listed, and
/// [`RealignError::Baseline`] when a baseline cannot be read or written.
/// Every baseline is read before any is written, and a failed write puts
/// back the baselines already written, so a failure leaves them as they
/// were; [`RealignError::LeftRealigned`] names any it could not put back.
pub fn realign_baselines(
    integrations_root: &Path,
    pre_migration: &[(PathBuf, String)],
    reader: &dyn FileReader,
    writer_at: &dyn Fn(&Path) -> Box<dyn AtomicWrite>,
    body_rules: &dyn Fn(&str) -> Option<&'static dyn BodyDialect>,
    parser: &dyn BodyParser,
) -> Result<usize, RealignError> {
    let mut by_id: HashMap<String, (&Path, &str)> = HashMap::new();
    for (path, original) in pre_migration {
        if let Some(id) = frontmatter_id(original) {
            by_id.insert(id, (path.as_path(), original.as_str()));
        }
    }

    let mut pending = Vec::new();
    for entry in
        fs::read_dir(integrations_root).map_err(RealignError::Listing)?
    {
        let entry = entry.map_err(RealignError::Listing)?;
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let integration = entry.file_name().to_string_lossy().into_owned();
        let Some(dialect) = body_rules(&integration) else {
            continue;
        };
        let digester = BaselineDigester::new(dialect, parser);
        pending.extend(
            Realignment::planned(
                baseline::path(integrations_root, &integration),
                writer_at(&entry.path()),
                &by_id,
                reader,
                &digester,
            )
            .map_err(RealignError::Baseline)?,
        );
    }
    apply_all_or_none(&pending, reader)
}

fn frontmatter_id(content: &str) -> Option<String> {
    let (frontmatter, _) = digest::split_frontmatter_and_body(content).ok()?;
    let entries = corpus::frontmatter_validation::parse_entries(&frontmatter);
    let raw = corpus::frontmatter_validation::raw_value(&entries, "id")?;
    let id = corpus::frontmatter_validation::strip_surrounding_quote(raw);
    (!id.is_empty()).then(|| id.to_owned())
}

struct Realignment {
    path: PathBuf,
    writer: Box<dyn AtomicWrite>,
    snapshot: String,
    realigned: Baseline,
    entries: usize,
}

impl Realignment {
    fn planned(
        path: PathBuf,
        writer: Box<dyn AtomicWrite>,
        by_id: &HashMap<String, (&Path, &str)>,
        reader: &dyn FileReader,
        digester: &BaselineDigester<'_>,
    ) -> Result<Option<Self>, StoreError> {
        let store = BaselineStore::new(path.clone(), reader, writer.as_ref());
        let Some(snapshot) = store.snapshot()? else {
            return Ok(None);
        };
        let (mut baseline, _) = Baseline::read(Some(&snapshot));
        let mut entries = 0;
        for (id, &(item, original)) in by_id {
            let Some(entry) = baseline.get(id) else {
                continue;
            };
            let Ok(Some(new_content)) = reader.read(item) else {
                continue;
            };
            if let Some(realigned_entry) =
                realigned_entry(entry, original, &new_content, digester)
            {
                baseline.set(id, realigned_entry);
                entries += 1;
            }
        }
        Ok((entries > 0).then_some(Self {
            path,
            writer,
            snapshot,
            realigned: baseline,
            entries,
        }))
    }

    fn store<'a>(&'a self, reader: &'a dyn FileReader) -> BaselineStore<'a> {
        BaselineStore::new(self.path.clone(), reader, self.writer.as_ref())
    }

    fn apply(&self, reader: &dyn FileReader) -> Result<(), StoreError> {
        self.store(reader).replace(&self.realigned)
    }

    fn undo(&self, reader: &dyn FileReader) -> Result<(), StoreError> {
        self.store(reader).put_back(&self.snapshot)
    }
}

fn apply_all_or_none(
    pending: &[Realignment],
    reader: &dyn FileReader,
) -> Result<usize, RealignError> {
    for (applied, realignment) in pending.iter().enumerate() {
        if let Err(cause) = realignment.apply(reader) {
            let left_realigned_baselines = pending[..applied]
                .iter()
                .filter(|earlier| earlier.undo(reader).is_err())
                .map(|earlier| earlier.path.clone())
                .collect::<Vec<_>>();
            return Err(if left_realigned_baselines.is_empty() {
                RealignError::Baseline(cause)
            } else {
                RealignError::LeftRealigned {
                    cause,
                    left_realigned_baselines,
                }
            });
        }
    }
    Ok(pending.iter().map(|realignment| realignment.entries).sum())
}

/// `entry` with its local digests moved to `new_content`'s, when they
/// recorded `original` and the migration moved them.
fn realigned_entry(
    entry: &baseline::Entry,
    original: &str,
    new_content: &str,
    digester: &BaselineDigester<'_>,
) -> Option<baseline::Entry> {
    match entry {
        baseline::Entry::Legacy { stamps, legacy } => {
            let pre_hash = digest::local(original).ok()?;
            let new_hash = digest::local(new_content).ok()?;
            (legacy.local.as_ref() == Some(&pre_hash) && new_hash != pre_hash)
                .then(|| baseline::Entry::Legacy {
                    stamps: stamps.clone(),
                    legacy: LegacyHashes {
                        local: Some(new_hash),
                        ..legacy.clone()
                    },
                })
        }
        baseline::Entry::Split {
            stamps,
            legacy,
            split,
        } => {
            let (_, pre_local) = digester.local(original).ok()?;
            let (new_hash, new_local) = digester.local(new_content).ok()?;
            (split.local.as_ref() == Some(&pre_local) && new_local != pre_local)
                .then(|| baseline::Entry::Split {
                    stamps: stamps.clone(),
                    legacy: LegacyHashes {
                        local: Some(new_hash),
                        ..legacy.clone()
                    },
                    split: SplitFingerprints {
                        local: Some(new_local),
                        ..split.clone()
                    },
                })
        }
        baseline::Entry::Unsupported { .. } => None,
    }
}
