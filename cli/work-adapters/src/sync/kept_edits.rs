//! Which local edits kept by earlier pulls are still needed.
//!
//! A copy stays until its item matches it again or a person deletes it,
//! whatever its age, because it may be the only copy of edits made since a
//! sync.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::LockName;
use corpus::store::CopyComparison;
use corpus::store::EntryKind;
use corpus::store::RecordedCopy;
use corpus::store::RecoveryCopies;
use corpus::store::StoreError;
use corpus::store::RECOVERY_BOOKKEEPING;
use corpus::WorkItemIdScheme;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use work::draft_id::is_token_safe;

use crate::sync::fetch::LocalItem;
use crate::sync::recovery_label::RecoveryLabel;
use crate::sync::recovery_label::PARENT;

/// One item's kept edits, located on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptCopy {
    pub id: String,
    pub item: PathBuf,
    pub copy: PathBuf,
    pub directory: PathBuf,
    /// The epoch the edits were kept at; `None` when unknown.
    pub kept_since: Option<u64>,
}

/// Why a directory under `sync-recovery` is not trusted as this tool's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// It has no readable record naming a copy of a work item, or holds a
    /// link.
    Unknown,
    /// Version control tracks something in it, so it came with the
    /// repository rather than from a pull.
    Tracked,
    /// Version control could not say whether it tracks it.
    TrackingUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeptEdits {
    KeptThisRun(KeptCopy),
    KeptEarlier(KeptCopy),
    /// The item matches its copy again, or the copy was deleted; the sweep
    /// removes it.
    Settled {
        kept: KeptCopy,
        recorded: RecordedCopy,
    },
    /// Never compared, swept or offered for restore.
    Unrecorded {
        id: Option<String>,
        directory: PathBuf,
        provenance: Provenance,
    },
    /// Settled, but its directory could not be removed.
    NotRemoved {
        kept: KeptCopy,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptEditsReview {
    pub entries: Vec<KeptEdits>,
}

pub struct KeptEditsRetention<'a> {
    pub recovery: &'a dyn RecoveryCopies,
    pub tracking: &'a dyn RepositoryTracking,
    pub locks: &'a dyn ExclusiveLock,
    pub work_root: PathBuf,
    pub scheme: &'a WorkItemIdScheme,
    pub corpus: &'a [LocalItem],
}

/// The files directly or deeply under a directory, and whether any entry
/// in it is a link.
struct Contents {
    files: Vec<PathBuf>,
    holds_a_link: bool,
}

impl KeptEditsRetention<'_> {
    /// Reads every kept-edits directory without changing any. `this_run`
    /// names the directories this run kept, which are reported and never
    /// compared.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when `sync-recovery` cannot be listed.
    pub fn review(
        &self,
        this_run: &BTreeSet<PathBuf>,
    ) -> Result<KeptEditsReview, StoreError> {
        let entries = self
            .recovery
            .all_entries(Path::new(PARENT))?
            .into_iter()
            .map(|entry| {
                if this_run.contains(&entry.path) {
                    self.kept_this_run(&entry.path)
                } else if entry.kind == EntryKind::Directory {
                    self.kept_earlier(&entry.path)
                } else {
                    self.unrecorded(&entry.path, Provenance::Unknown)
                }
            })
            .collect();
        Ok(KeptEditsReview { entries })
    }

    /// Removes exactly the directories `review` found settled, each only
    /// while its item's lock is held and it is still settled.
    #[must_use]
    pub fn sweep(&self, review: KeptEditsReview) -> KeptEditsReview {
        let entries = review
            .entries
            .into_iter()
            .filter_map(|entry| match entry {
                KeptEdits::Settled { kept, recorded } => {
                    self.remove_settled(kept, &recorded)
                }
                other => Some(other),
            })
            .collect();
        KeptEditsReview { entries }
    }

    fn remove_settled(
        &self,
        kept: KeptCopy,
        recorded: &RecordedCopy,
    ) -> Option<KeptEdits> {
        let Ok(_held) = self
            .locks
            .acquire(&LockName::ForFile(recorded.original.clone()))
        else {
            return Some(KeptEdits::KeptEarlier(kept));
        };
        if !self.still_settled(recorded) {
            return Some(KeptEdits::KeptEarlier(kept));
        }
        match self.recovery.remove_dir(&recorded.dir) {
            Ok(()) => None,
            Err(error) => Some(KeptEdits::NotRemoved {
                kept,
                reason: error.to_string(),
            }),
        }
    }

    fn kept_this_run(&self, dir: &Path) -> KeptEdits {
        let label = label_of(dir);
        match (self.recovery.recorded_copy(dir), label) {
            (Some(recorded), Some(label)) => {
                KeptEdits::KeptThisRun(self.located(&label, &recorded))
            }
            _ => self.unrecorded(dir, Provenance::Unknown),
        }
    }

    fn kept_earlier(&self, dir: &Path) -> KeptEdits {
        let Some(label) = label_of(dir)
            .filter(|label| is_token_safe(self.scheme, label.id()))
        else {
            return self.unrecorded(dir, Provenance::Unknown);
        };
        let Ok(contents) = self.contents(dir) else {
            return self.unrecorded(dir, Provenance::Unknown);
        };
        if contents.holds_a_link {
            return self.unrecorded(dir, Provenance::Unknown);
        }
        if let Some(provenance) = self.version_controlled(&contents) {
            return self.unrecorded(dir, provenance);
        }
        match self.recovery.recorded_copy(dir) {
            Some(recorded) if self.under_work_root(&recorded.original) => {
                self.judged(&label, recorded, &contents)
            }
            Some(_) => self.unrecorded(dir, Provenance::Unknown),
            None => self.matched_by_mirror(dir, &label),
        }
    }

    fn judged(
        &self,
        label: &RecoveryLabel,
        recorded: RecordedCopy,
        contents: &Contents,
    ) -> KeptEdits {
        let kept = self.located(label, &recorded);
        if self.settles(&recorded, contents) {
            KeptEdits::Settled { kept, recorded }
        } else {
            KeptEdits::KeptEarlier(kept)
        }
    }

    /// A copy written before its record was: found where a pull would have
    /// mirrored the item named by the directory, and never swept.
    fn matched_by_mirror(
        &self,
        dir: &Path,
        label: &RecoveryLabel,
    ) -> KeptEdits {
        self.corpus
            .iter()
            .find(|item| item.id == label.id())
            .map(|item| self.recovery.mirrored_copy(dir, &item.path))
            .filter(|recorded| {
                self.recovery.compare_copy(recorded)
                    != CopyComparison::CopyMissing
            })
            .map_or_else(
                || self.unrecorded(dir, Provenance::Unknown),
                |recorded| {
                    KeptEdits::KeptEarlier(self.located(label, &recorded))
                },
            )
    }

    fn settles(&self, recorded: &RecordedCopy, contents: &Contents) -> bool {
        match self.recovery.compare_copy(recorded) {
            CopyComparison::Equal => true,
            CopyComparison::CopyMissing => holds_only_bookkeeping(contents),
            CopyComparison::Different
            | CopyComparison::CopyUnreadable
            | CopyComparison::OriginalUnreadable => false,
        }
    }

    fn still_settled(&self, recorded: &RecordedCopy) -> bool {
        self.contents(&recorded.dir).is_ok_and(|contents| {
            !contents.holds_a_link && self.settles(recorded, &contents)
        })
    }

    fn version_controlled(&self, contents: &Contents) -> Option<Provenance> {
        let answers = contents.files.iter().map(|file| {
            self.tracking.file_tracking(&self.recovery.location(file))
        });
        match FileTracking::combine(answers) {
            FileTracking::Tracked => Some(Provenance::Tracked),
            FileTracking::Unknown => Some(Provenance::TrackingUnknown),
            FileTracking::Untracked => None,
        }
    }

    fn contents(&self, dir: &Path) -> Result<Contents, StoreError> {
        let mut contents = Contents {
            files: Vec::new(),
            holds_a_link: false,
        };
        let mut pending = vec![dir.to_path_buf()];
        while let Some(next) = pending.pop() {
            for entry in self.recovery.all_entries(&next)? {
                match entry.kind {
                    EntryKind::Directory => pending.push(entry.path),
                    EntryKind::File => contents.files.push(entry.path),
                    EntryKind::Symlink => contents.holds_a_link = true,
                }
            }
        }
        Ok(contents)
    }

    fn under_work_root(&self, original: &Path) -> bool {
        let canonical = |path: &Path| {
            let parent = path.parent()?.canonicalize().ok()?;
            Some(parent.join(path.file_name()?))
        };
        let work_root = self.work_root.canonicalize().ok();
        canonical(original)
            .zip(work_root)
            .is_some_and(|(original, root)| original.starts_with(root))
    }

    fn located(
        &self,
        label: &RecoveryLabel,
        recorded: &RecordedCopy,
    ) -> KeptCopy {
        KeptCopy {
            id: label.id().to_owned(),
            item: recorded.original.clone(),
            copy: self.recovery.location(&recorded.copy_path()),
            directory: self.recovery.location(&recorded.dir),
            kept_since: label.epoch(),
        }
    }

    fn unrecorded(&self, dir: &Path, provenance: Provenance) -> KeptEdits {
        KeptEdits::Unrecorded {
            id: label_of(dir)
                .filter(|label| is_token_safe(self.scheme, label.id()))
                .map(|label| label.id().to_owned()),
            directory: self.recovery.location(dir),
            provenance,
        }
    }
}

fn label_of(dir: &Path) -> Option<RecoveryLabel> {
    RecoveryLabel::parse(dir.file_name()?.to_str()?)
}

fn holds_only_bookkeeping(contents: &Contents) -> bool {
    contents.files.iter().all(|file| {
        file.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| RECOVERY_BOOKKEEPING.contains(&name))
    })
}
