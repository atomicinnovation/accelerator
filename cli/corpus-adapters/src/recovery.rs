//! The filesystem [`RecoveryCopies`]: copies under a state directory outside
//! the corpus, each at its original's corpus-relative position.

use std::fs;
use std::io::{ErrorKind, Write as _};
use std::path::{Component, Path, PathBuf};

use corpus::store::KeptRecovery;
use corpus::store::KeptState;
use corpus::store::RecoveryCopies;
use corpus::StoreError;

const IGNORE_EVERYTHING: &str = "*\n";
const RESTORE_PENDING: &str = "RESTORE-PENDING";
const COMPLETED: &str = "COMPLETED";

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

    #[must_use]
    pub fn location(&self, dir: &Path) -> PathBuf {
        self.state_root.join(dir)
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
        let relative = original.strip_prefix(&self.corpus_root).map_or_else(
            |_| {
                original
                    .components()
                    .filter(|component| {
                        matches!(component, Component::Normal(_))
                    })
                    .collect()
            },
            Path::to_path_buf,
        );
        self.location(dir).join(relative)
    }
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
        let ignore = location.join(".gitignore");
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
        fs::write(&marker, listing).map_err(|error| io(&marker, &error))
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

    fn copy_settled(&self, dir: &Path, original: &Path) -> bool {
        let Ok(copy) = fs::read(self.mirrored(dir, original)) else {
            return true;
        };
        fs::read(original).is_ok_and(|current| current == copy)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use corpus::store::RecoveryCopies as _;
    use corpus::StoreError;
    use tempfile::TempDir;

    use super::FileRecoveryCopies;

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
    fn a_copy_is_settled_once_its_original_matches_it_or_it_is_deleted(
    ) -> Result<(), TestError> {
        let state = TempDir::new()?;
        let corpus = TempDir::new()?;
        let store = FileRecoveryCopies::new(state.path(), corpus.path());
        let original = corpus.path().join("work/a.md");
        fs::create_dir_all(original.parent().ok_or("no parent")?)?;
        fs::write(&original, "edited")?;
        store.prepare(dir())?;
        store.write_once(dir(), &original, b"before")?;

        assert!(!store.copy_settled(dir(), &original));
        fs::write(&original, "before")?;
        assert!(store.copy_settled(dir(), &original));
        fs::write(&original, "edited")?;
        fs::remove_file(state.path().join(dir()).join("work/a.md"))?;
        assert!(store.copy_settled(dir(), &original));
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
}
