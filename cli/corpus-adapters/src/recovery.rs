//! The filesystem [`RecoveryCopies`]: copies under a state directory outside
//! the corpus, each at its original's corpus-relative position.

use std::fs;
use std::io::{ErrorKind, Write as _};
use std::path::{Component, Path, PathBuf};

use corpus::store::CopyComparison;
use corpus::store::EntryKind;
use corpus::store::KeptRecovery;
use corpus::store::KeptState;
use corpus::store::RecordedCopy;
use corpus::store::RecoveryCopies;
use corpus::store::RecoveryEntry;
use corpus::store::RECOVERY_BOOKKEEPING;
use corpus::StoreError;

const IGNORE_EVERYTHING: &str = "*\n";
const IGNORE_RULE: &str = RECOVERY_BOOKKEEPING[0];
const RESTORE_PENDING: &str = RECOVERY_BOOKKEEPING[1];
const COMPLETED: &str = "COMPLETED";
const RECORD_STAGING: &str = ".RESTORE-PENDING.staged";

pub struct FileRecoveryCopies {
    state_root: PathBuf,
    corpus_root: PathBuf,
}

impl FileRecoveryCopies {
    #[must_use]
    pub fn new(
        state_root: impl Into<PathBuf>,
        corpus_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            state_root: state_root.into(),
            corpus_root: corpus_root.into(),
        }
    }

    fn contained(&self, dir: &Path) -> Result<PathBuf, StoreError> {
        if dir
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            Ok(self.location(dir))
        } else {
            Err(StoreError::Validation {
                detail: format!(
                    "{} would reach outside the recovery state directory",
                    dir.display()
                ),
            })
        }
    }

    fn mirrored(&self, dir: &Path, original: &Path) -> PathBuf {
        self.location(dir).join(self.mirrored_relative(original))
    }

    fn mirrored_relative(&self, original: &Path) -> PathBuf {
        original.strip_prefix(&self.corpus_root).map_or_else(
            |_| {
                original
                    .components()
                    .filter(|component| {
                        matches!(component, Component::Normal(_))
                    })
                    .collect()
            },
            Path::to_path_buf,
        )
    }

    /// `dir` on disk, when it is a real directory rather than a link.
    fn real_directory(&self, dir: &Path) -> Option<PathBuf> {
        let location = self.contained(dir).ok()?;
        fs::symlink_metadata(&location)
            .is_ok_and(|metadata| metadata.is_dir())
            .then_some(location)
    }
}

fn only_normal_components(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

type RecordRename = fn(&Path, &Path) -> std::io::Result<()>;

/// Stages the whole record and renames it into place, so a crash leaves the
/// previous record or the new one, never a truncated one.
fn write_record(
    record: &Path,
    contents: &str,
    rename: RecordRename,
) -> Result<(), StoreError> {
    let staged = record.with_file_name(RECORD_STAGING);
    let written = fs::File::create(&staged).and_then(|mut file| {
        file.write_all(contents.as_bytes())
            .and_then(|()| file.sync_all())
    });
    written
        .and_then(|()| rename(&staged, record))
        .map_err(|error| {
            let _ = fs::remove_file(&staged);
            io(record, &error)
        })
}

fn parse_copy_record(dir: &Path, record: &str) -> Option<RecordedCopy> {
    let mut lines = record.lines();
    let (original, copy) = lines.next()?.split_once('\t')?;
    let copy = PathBuf::from(copy);
    (lines.next().is_none() && only_normal_components(&copy)).then(|| {
        RecordedCopy {
            dir: dir.to_path_buf(),
            original: PathBuf::from(original),
            copy,
        }
    })
}

fn read_unlinked(path: &Path) -> Result<Vec<u8>, ErrorKind> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.kind())?;
    if !metadata.is_file() {
        return Err(ErrorKind::InvalidInput);
    }
    fs::read(path).map_err(|error| error.kind())
}

fn io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}

fn write_durably(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == ErrorKind::AlreadyExists {
                StoreError::AlreadyExists {
                    path: path.display().to_string(),
                }
            } else {
                io(path, &error)
            }
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io(path, &error))
}

impl RecoveryCopies for FileRecoveryCopies {
    fn exists(&self, dir: &Path) -> bool {
        self.location(dir).is_dir()
    }

    fn prepare(&self, dir: &Path) -> Result<(), StoreError> {
        let location = self.contained(dir)?;
        fs::create_dir_all(&location).map_err(|error| io(&location, &error))?;
        let ignore = location.join(IGNORE_RULE);
        fs::write(&ignore, IGNORE_EVERYTHING)
            .map_err(|error| io(&ignore, &error))?;
        let verified =
            fs::read_to_string(&ignore).map_err(|error| io(&ignore, &error))?;
        if verified == IGNORE_EVERYTHING {
            Ok(())
        } else {
            Err(StoreError::Io {
                path: ignore.display().to_string(),
                detail: "the ignore rule could not be verified".to_owned(),
            })
        }
    }

    fn write_once(
        &self,
        dir: &Path,
        original: &Path,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.contained(dir)?;
        let copy = self.mirrored(dir, original);
        if let Some(parent) = copy.parent() {
            fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
        }
        write_durably(&copy, bytes)
    }

    fn mark_restore_pending(
        &self,
        dir: &Path,
        unrestored: &[PathBuf],
    ) -> Result<(), StoreError> {
        let marker = self.contained(dir)?.join(RESTORE_PENDING);
        let listing = unrestored.iter().fold(String::new(), |listing, path| {
            listing + &path.display().to_string() + "\n"
        });
        write_record(&marker, &listing, |from, to| fs::rename(from, to))
    }

    fn is_restore_pending(&self, dir: &Path) -> bool {
        self.location(dir).join(RESTORE_PENDING).is_file()
    }

    fn mark_completed(&self, dir: &Path) -> Result<(), StoreError> {
        let location = self.contained(dir)?;
        let notice = location.join(COMPLETED);
        fs::write(
            &notice,
            "The change these copies were kept for has since completed. \
             They remain for reference; compare each with its original and \
             delete this directory when done.\n",
        )
        .map_err(|error| io(&notice, &error))?;
        let marker = location.join(RESTORE_PENDING);
        fs::remove_file(&marker).map_err(|error| io(&marker, &error))
    }

    fn remove_dir(&self, dir: &Path) -> Result<(), StoreError> {
        let location = self.contained(dir)?;
        if fs::symlink_metadata(&location)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return fs::remove_file(&location)
                .map_err(|error| io(&location, &error));
        }
        match fs::remove_dir_all(&location) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(io(&location, &error))
            }
            _ => Ok(()),
        }
    }

    fn kept(&self, parent: &Path) -> Result<Vec<KeptRecovery>, StoreError> {
        let location = self.location(parent);
        let entries = match fs::read_dir(&location) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(io(&location, &error)),
        };
        let mut names: Vec<_> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name())
            .collect();
        names.sort();
        Ok(names
            .into_iter()
            .filter_map(|name| {
                let dir = parent.join(name);
                let found = self.location(&dir);
                let state = if found.join(COMPLETED).is_file() {
                    KeptState::Completed
                } else {
                    let listing =
                        fs::read_to_string(found.join(RESTORE_PENDING)).ok()?;
                    KeptState::RestorePending(
                        listing.lines().map(PathBuf::from).collect(),
                    )
                };
                Some(KeptRecovery { dir, state })
            })
            .collect())
    }

    fn all_entries(
        &self,
        parent: &Path,
    ) -> Result<Vec<RecoveryEntry>, StoreError> {
        let location = self.contained(parent)?;
        let entries = match fs::read_dir(&location) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(io(&location, &error)),
        };
        let mut listed: Vec<RecoveryEntry> = entries
            .map(|entry| {
                let entry = entry.map_err(|error| io(&location, &error))?;
                let kind = entry
                    .file_type()
                    .map_err(|error| io(&entry.path(), &error))?;
                Ok(RecoveryEntry {
                    path: parent.join(entry.file_name()),
                    kind: if kind.is_symlink() {
                        EntryKind::Symlink
                    } else if kind.is_dir() {
                        EntryKind::Directory
                    } else {
                        EntryKind::File
                    },
                })
            })
            .collect::<Result<_, StoreError>>()?;
        listed.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(listed)
    }

    fn location(&self, path: &Path) -> PathBuf {
        self.state_root.join(path)
    }

    fn mirrored_copy(&self, dir: &Path, original: &Path) -> RecordedCopy {
        RecordedCopy {
            dir: dir.to_path_buf(),
            original: original.to_path_buf(),
            copy: self.mirrored_relative(original),
        }
    }

    fn mark_copy_pending(&self, copy: &RecordedCopy) -> Result<(), StoreError> {
        let record = self.contained(&copy.dir)?.join(RESTORE_PENDING);
        write_record(
            &record,
            &format!("{}\t{}\n", copy.original.display(), copy.copy.display()),
            |from, to| fs::rename(from, to),
        )
    }

    fn recorded_copy(&self, dir: &Path) -> Option<RecordedCopy> {
        let record = self.real_directory(dir)?.join(RESTORE_PENDING);
        let bytes = read_unlinked(&record).ok()?;
        parse_copy_record(dir, std::str::from_utf8(&bytes).ok()?)
    }

    fn compare_copy(&self, copy: &RecordedCopy) -> CopyComparison {
        let Some(dir) = self.real_directory(&copy.dir) else {
            return CopyComparison::CopyUnreadable;
        };
        if !only_normal_components(&copy.copy) {
            return CopyComparison::CopyUnreadable;
        }
        let kept = match read_unlinked(&dir.join(&copy.copy)) {
            Ok(kept) => kept,
            Err(ErrorKind::NotFound) => return CopyComparison::CopyMissing,
            Err(_) => return CopyComparison::CopyUnreadable,
        };
        match fs::read(&copy.original) {
            Ok(current) if current == kept => CopyComparison::Equal,
            Ok(_) => CopyComparison::Different,
            Err(_) => CopyComparison::OriginalUnreadable,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use corpus::store::CopyComparison;
    use corpus::store::EntryKind;
    use corpus::store::RecordedCopy;
    use corpus::store::RecoveryCopies as _;
    use corpus::store::RecoveryEntry;
    use corpus::StoreError;
    use tempfile::TempDir;

    use super::write_record;
    use super::FileRecoveryCopies;
    use super::RecordRename;

    type TestError = Box<dyn std::error::Error>;

    fn copies(state: &TempDir) -> FileRecoveryCopies {
        FileRecoveryCopies::new(state.path(), "/repo/meta")
    }

    fn dir() -> &'static Path {
        Path::new("retirement-recovery/draft-k7mq3x--PP-900")
    }

    #[test]
    fn a_directory_escaping_the_state_root_is_never_written_or_removed(
    ) -> Result<(), TestError> {
        let parent = TempDir::new()?;
        let state = parent.path().join("state");
        let victim = parent.path().join("victim");
        fs::create_dir_all(&victim)?;
        let store = FileRecoveryCopies::new(&state, "/repo/meta");
        let escaping = Path::new("retirement-recovery/PP-1--ENG-1/../../..");

        assert!(store.prepare(escaping).is_err());
        assert!(store.mark_completed(escaping).is_err());
        assert!(store.remove_dir(escaping).is_err());
        assert!(victim.is_dir(), "nothing outside the state root is removed");
        Ok(())
    }

    #[test]
    fn kept_lists_restore_pending_and_completed_directories(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let store = copies(&state);
        let pending = Path::new("retirement-recovery/PP-1--ENG-1");
        let completed = Path::new("retirement-recovery/PP-2--ENG-2");
        let plain = Path::new("retirement-recovery/PP-3--ENG-3");
        for dir in [pending, completed, plain] {
            store.prepare(dir)?;
        }
        store.mark_restore_pending(
            pending,
            &[PathBuf::from("/repo/meta/work/a.md")],
        )?;
        store.mark_restore_pending(completed, &[])?;
        store.mark_completed(completed)?;

        let kept = store.kept(Path::new("retirement-recovery"))?;

        assert_eq!(
            kept,
            vec![
                corpus::store::KeptRecovery {
                    dir: pending.to_path_buf(),
                    state: corpus::store::KeptState::RestorePending(vec![
                        PathBuf::from("/repo/meta/work/a.md")
                    ]),
                },
                corpus::store::KeptRecovery {
                    dir: completed.to_path_buf(),
                    state: corpus::store::KeptState::Completed,
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn a_copy_mirrors_its_originals_corpus_relative_path(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let store = copies(&state);
        store.prepare(dir())?;
        store.write_once(
            dir(),
            Path::new("/repo/meta/work/drafts/draft-k7mq3x-a.md"),
            b"draft",
        )?;
        assert_eq!(
            fs::read(
                state
                    .path()
                    .join(dir())
                    .join("work/drafts/draft-k7mq3x-a.md")
            )?,
            b"draft"
        );
        Ok(())
    }

    #[test]
    fn prepare_writes_an_ignore_rule_for_everything() -> Result<(), TestError> {
        let state = TempDir::new()?;
        copies(&state).prepare(dir())?;
        assert_eq!(
            fs::read_to_string(state.path().join(dir()).join(".gitignore"))?,
            "*\n"
        );
        Ok(())
    }

    #[test]
    fn recovery_copies_refuse_to_overwrite_an_existing_copy(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let store = copies(&state);
        let original = Path::new("/repo/meta/work/0001-a.md");
        store.prepare(dir())?;
        store.write_once(dir(), original, b"before the crash")?;
        let second = store.write_once(dir(), original, b"after");
        assert!(matches!(second, Err(StoreError::AlreadyExists { .. })));
        assert_eq!(
            fs::read(state.path().join(dir()).join("work/0001-a.md"))?,
            b"before the crash"
        );
        Ok(())
    }

    #[test]
    fn a_failed_ignore_rule_write_aborts_before_any_copy(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let location = state.path().join(dir());
        fs::create_dir_all(location.join(".gitignore"))?;
        let result = copies(&state).prepare(dir());
        assert!(result.is_err());
        let entries: Vec<PathBuf> = fs::read_dir(&location)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        assert_eq!(entries, vec![location.join(".gitignore")]);
        Ok(())
    }

    #[test]
    fn a_restore_pending_marker_is_downgraded_to_a_completion_notice(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let store = copies(&state);
        store.prepare(dir())?;
        store.mark_restore_pending(
            dir(),
            &[PathBuf::from("/repo/meta/work/0001-a.md")],
        )?;
        assert!(store.is_restore_pending(dir()));
        store.mark_completed(dir())?;
        assert!(!store.is_restore_pending(dir()));
        assert!(state.path().join(dir()).join("COMPLETED").is_file());
        Ok(())
    }

    #[test]
    fn removing_an_absent_directory_is_not_an_error() -> Result<(), TestError> {
        let state = TempDir::new()?;
        copies(&state).remove_dir(dir())?;
        Ok(())
    }

    fn pull_dir() -> &'static Path {
        Path::new("sync-recovery/1700000000-k7mq3x-0001")
    }

    struct Kept {
        state: TempDir,
        corpus: TempDir,
        store: FileRecoveryCopies,
        original: PathBuf,
    }

    fn kept_copy(original: &str, copy: &str) -> Result<Kept, TestError> {
        let state = TempDir::new()?;
        let corpus = TempDir::new()?;
        let store = FileRecoveryCopies::new(state.path(), corpus.path());
        let path = corpus.path().join("work/0001.md");
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(&path, original)?;
        store.prepare(pull_dir())?;
        store.write_once(pull_dir(), &path, copy.as_bytes())?;
        Ok(Kept {
            state,
            corpus,
            store,
            original: path,
        })
    }

    impl Kept {
        fn copy(&self) -> RecordedCopy {
            self.store.mirrored_copy(pull_dir(), &self.original)
        }

        fn compare(&self) -> CopyComparison {
            self.store.compare_copy(&self.copy())
        }
    }

    #[test]
    fn a_copy_equal_to_its_original_compares_equal() -> Result<(), TestError> {
        assert_eq!(kept_copy("same", "same")?.compare(), CopyComparison::Equal);
        Ok(())
    }

    #[test]
    fn a_copy_unlike_its_original_compares_different() -> Result<(), TestError>
    {
        assert_eq!(
            kept_copy("edited", "before")?.compare(),
            CopyComparison::Different
        );
        Ok(())
    }

    #[test]
    fn a_deleted_copy_compares_missing() -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        fs::remove_file(kept.store.location(&kept.copy().copy_path()))?;

        assert_eq!(kept.compare(), CopyComparison::CopyMissing);
        Ok(())
    }

    #[test]
    fn a_copy_that_cannot_be_read_compares_unreadable() -> Result<(), TestError>
    {
        let kept = kept_copy("edited", "before")?;
        let copy = kept.store.location(&kept.copy().copy_path());
        fs::remove_file(&copy)?;
        fs::create_dir(&copy)?;

        assert_eq!(kept.compare(), CopyComparison::CopyUnreadable);
        Ok(())
    }

    #[test]
    fn an_original_that_cannot_be_read_compares_unreadable(
    ) -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        fs::remove_file(&kept.original)?;

        assert_eq!(kept.compare(), CopyComparison::OriginalUnreadable);
        Ok(())
    }

    #[test]
    fn a_copy_verdict_wins_when_neither_side_can_be_read(
    ) -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        fs::remove_file(&kept.original)?;
        fs::remove_file(kept.store.location(&kept.copy().copy_path()))?;

        assert_eq!(kept.compare(), CopyComparison::CopyMissing);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_copy_is_never_read() -> Result<(), TestError> {
        let kept = kept_copy("same", "same")?;
        let copy = kept.store.location(&kept.copy().copy_path());
        fs::remove_file(&copy)?;
        std::os::unix::fs::symlink(&kept.original, &copy)?;

        assert_eq!(kept.compare(), CopyComparison::CopyUnreadable);
        Ok(())
    }

    #[test]
    fn a_copy_record_names_where_its_copy_was_written() -> Result<(), TestError>
    {
        let kept = kept_copy("edited", "before")?;
        let copy = kept.copy();

        kept.store.mark_copy_pending(&copy)?;

        assert_eq!(kept.store.recorded_copy(pull_dir()), Some(copy));
        assert_eq!(
            kept.store.recorded_copy(pull_dir()).map(|copy| copy.copy),
            Some(PathBuf::from("work/0001.md"))
        );
        drop(kept.corpus);
        Ok(())
    }

    #[test]
    fn a_retirement_record_names_no_copy() -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        kept.store.mark_restore_pending(
            pull_dir(),
            std::slice::from_ref(&kept.original),
        )?;

        assert_eq!(kept.store.recorded_copy(pull_dir()), None);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_record_is_unrecorded() -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        kept.store.mark_copy_pending(&kept.copy())?;
        let record = kept.state.path().join(pull_dir()).join("RESTORE-PENDING");
        let elsewhere = kept.state.path().join("elsewhere");
        fs::rename(&record, &elsewhere)?;
        std::os::unix::fs::symlink(&elsewhere, &record)?;

        assert_eq!(kept.store.recorded_copy(pull_dir()), None);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_recovery_directory_is_unrecorded() -> Result<(), TestError> {
        let kept = kept_copy("edited", "before")?;
        kept.store.mark_copy_pending(&kept.copy())?;
        let dir = kept.state.path().join(pull_dir());
        let elsewhere = kept.state.path().join("elsewhere");
        fs::rename(&dir, &elsewhere)?;
        std::os::unix::fs::symlink(&elsewhere, &dir)?;

        assert_eq!(kept.store.recorded_copy(pull_dir()), None);
        Ok(())
    }

    #[test]
    fn all_entries_lists_record_less_entries() -> Result<(), TestError> {
        let state = TempDir::new()?;
        let store = copies(&state);
        let recorded = Path::new("sync-recovery/1-k7mq3x-0001");
        let bare = Path::new("sync-recovery/2-k7mq3x-0002");
        store.prepare(recorded)?;
        store.mark_restore_pending(recorded, &[])?;
        fs::create_dir_all(state.path().join(bare))?;
        fs::write(state.path().join("sync-recovery/stray"), "")?;

        let entries = store.all_entries(Path::new("sync-recovery"))?;

        assert_eq!(
            entries,
            vec![
                RecoveryEntry {
                    path: recorded.to_path_buf(),
                    kind: EntryKind::Directory,
                },
                RecoveryEntry {
                    path: bare.to_path_buf(),
                    kind: EntryKind::Directory,
                },
                RecoveryEntry {
                    path: PathBuf::from("sync-recovery/stray"),
                    kind: EntryKind::File,
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn all_entries_of_a_missing_parent_is_empty() -> Result<(), TestError> {
        let state = TempDir::new()?;

        assert_eq!(
            copies(&state).all_entries(Path::new("sync-recovery"))?,
            Vec::new()
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn all_entries_reports_a_symlink_without_following_it(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let outside = TempDir::new()?;
        fs::create_dir_all(state.path().join("sync-recovery"))?;
        std::os::unix::fs::symlink(
            outside.path(),
            state.path().join("sync-recovery/linked"),
        )?;

        assert_eq!(
            copies(&state).all_entries(Path::new("sync-recovery"))?,
            vec![RecoveryEntry {
                path: PathBuf::from("sync-recovery/linked"),
                kind: EntryKind::Symlink,
            }]
        );
        Ok(())
    }

    #[test]
    fn all_entries_refuses_an_escaping_parent() -> Result<(), TestError> {
        let state = TempDir::new()?;

        assert!(copies(&state)
            .all_entries(Path::new("sync-recovery/../.."))
            .is_err());
        Ok(())
    }

    #[test]
    fn a_restore_pending_record_survives_a_fault_after_the_temp_write(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let record = state.path().join("RESTORE-PENDING");
        fs::write(&record, "the previous record\n")?;
        let failing_rename: RecordRename =
            |_, _| Err(std::io::Error::other("injected rename failure"));

        let outcome = write_record(&record, "the new record\n", failing_rename);

        assert!(outcome.is_err());
        assert_eq!(fs::read_to_string(&record)?, "the previous record\n");
        Ok(())
    }
}
