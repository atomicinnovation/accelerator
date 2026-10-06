//! Whether a file is tracked, answered so that it fails closed.
//!
//! [`crate::facts`] is unfit here: it lets `.jj` beat `.git`, and git lets a
//! repository commit a `.jj` directory, so a crafted marker would decide which
//! VCS is asked. This walk instead finds the nearest `.jj` and the nearest
//! `.git` independently and asks every repository that encloses the file. A
//! repository that tracks the file wins; one that cannot answer makes the
//! whole answer `Unknown` unless another tracks it.
//!
//! A genuine jj store is never tracked by git, even when colocated, so a `.jj`
//! directory with any file in the enclosing git index is itself suspect: it
//! trips the answer to `Unknown`.

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
use vcs::VcsKind;

use crate::library::Error;
use crate::library::InProcessProbe;
use crate::roots::repository_roots;

/// Answers the tracking questions by reading every enclosing repository in
/// the calling process.
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessTracking;

impl RepositoryTracking for InProcessTracking {
    fn file_tracking(&self, path: &Path) -> FileTracking {
        file_tracking(path)
    }

    fn repository_roots(&self, directory: &Path) -> RootsAnswer {
        repository_roots(directory)
    }
}

/// The tracking status of `path` across every repository enclosing it.
#[must_use]
pub fn file_tracking(path: &Path) -> FileTracking {
    file_tracking_with(path, &FilesystemMarkers, &InProcessProbe)
}

/// Whether a marker entry exists. An error means a marker may be there but
/// cannot be seen.
pub trait MarkerProbe {
    fn present(&self, marker: &Path) -> io::Result<bool>;
}

pub struct FilesystemMarkers;

impl MarkerProbe for FilesystemMarkers {
    fn present(&self, marker: &Path) -> io::Result<bool> {
        match fs::symlink_metadata(marker) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

/// The two index queries the walk puts to each repository.
trait TrackingQueries {
    fn is_tracked(
        &self,
        root: &Path,
        relpath: &str,
        kind: VcsKind,
    ) -> Result<bool, Error>;

    fn tracks_any_under(
        &self,
        root: &Path,
        directory: &str,
    ) -> Result<bool, Error>;
}

impl TrackingQueries for InProcessProbe {
    fn is_tracked(
        &self,
        root: &Path,
        relpath: &str,
        kind: VcsKind,
    ) -> Result<bool, Error> {
        Self::is_tracked(root, relpath, kind)
    }

    fn tracks_any_under(
        &self,
        root: &Path,
        directory: &str,
    ) -> Result<bool, Error> {
        Self::tracks_any_under(root, directory)
    }
}

/// A repository found above the file, or a marker that could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Enclosing {
    Repository { root: PathBuf, kind: VcsKind },
    Unusable,
}

fn file_tracking_with(
    path: &Path,
    markers: &dyn MarkerProbe,
    queries: &dyn TrackingQueries,
) -> FileTracking {
    let Ok(file) = dunce::canonicalize(path) else {
        return FileTracking::Unknown;
    };
    let Some(directory) = file.parent() else {
        return FileTracking::Unknown;
    };
    let enclosing = enclosing_repositories(directory, markers);
    let answers = enclosing
        .iter()
        .map(|repository| answer(repository, &file, queries));
    let tripped = tripwire(&enclosing, queries);
    FileTracking::combine(answers.chain(tripped))
}

/// The nearest `.jj` and the nearest `.git` at or above `directory`, each
/// found independently so neither marker can hide the other.
pub fn enclosing_repositories(
    directory: &Path,
    markers: &dyn MarkerProbe,
) -> Vec<Enclosing> {
    [(".jj", VcsKind::Jj), (".git", VcsKind::Git)]
        .into_iter()
        .filter_map(|(marker, kind)| nearest(directory, marker, kind, markers))
        .collect()
}

fn nearest(
    directory: &Path,
    marker: &str,
    kind: VcsKind,
    markers: &dyn MarkerProbe,
) -> Option<Enclosing> {
    for dir in directory.ancestors() {
        match markers.present(&dir.join(marker)) {
            Ok(false) => {}
            Ok(true) if dir.to_str().is_some() => {
                return Some(Enclosing::Repository {
                    root: dir.to_path_buf(),
                    kind,
                })
            }
            Ok(true) | Err(_) => return Some(Enclosing::Unusable),
        }
    }
    None
}

fn answer(
    enclosing: &Enclosing,
    file: &Path,
    queries: &dyn TrackingQueries,
) -> FileTracking {
    let Enclosing::Repository { root, kind } = enclosing else {
        return FileTracking::Unknown;
    };
    let Some(relpath) = file.strip_prefix(root).ok().and_then(Path::to_str)
    else {
        return FileTracking::Unknown;
    };
    match queries.is_tracked(root, relpath, *kind) {
        Ok(true) => FileTracking::Tracked,
        Ok(false) => FileTracking::Untracked,
        Err(_) => FileTracking::Unknown,
    }
}

fn tripwire(
    enclosing: &[Enclosing],
    queries: &dyn TrackingQueries,
) -> Option<FileTracking> {
    let root_of = |wanted: VcsKind| {
        enclosing.iter().find_map(|repository| match repository {
            Enclosing::Repository { root, kind } if *kind == wanted => {
                Some(root)
            }
            _ => None,
        })
    };
    let (git, jj) = (root_of(VcsKind::Git)?, root_of(VcsKind::Jj)?);
    let store = jj.strip_prefix(git).ok()?.join(".jj");
    let Some(store) = store.to_str() else {
        return Some(FileTracking::Unknown);
    };
    match queries.tracks_any_under(git, store) {
        Ok(false) => None,
        Ok(true) | Err(_) => Some(FileTracking::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::path::Path;
    use std::path::PathBuf;

    use vcs::tracking::FileTracking;
    use vcs::VcsKind;

    use super::{file_tracking_with, MarkerProbe, TrackingQueries};
    use crate::library::Error;

    struct Markers {
        present: Vec<PathBuf>,
        failing: Option<PathBuf>,
    }

    impl MarkerProbe for Markers {
        fn present(&self, marker: &Path) -> io::Result<bool> {
            if self.failing.as_deref() == Some(marker) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.present.iter().any(|present| present == marker))
        }
    }

    struct Queries {
        jj: Result<bool, ()>,
        git: Result<bool, ()>,
        git_tracks_the_store: bool,
        asked_under: RefCell<Vec<String>>,
    }

    fn failure(root: &Path) -> Error {
        Error::JjStoreLayout {
            store: root.to_path_buf(),
        }
    }

    impl TrackingQueries for Queries {
        fn is_tracked(
            &self,
            root: &Path,
            _relpath: &str,
            kind: VcsKind,
        ) -> Result<bool, Error> {
            match kind {
                VcsKind::Jj => self.jj,
                VcsKind::Git => self.git,
                VcsKind::None => Ok(false),
            }
            .map_err(|()| failure(root))
        }

        fn tracks_any_under(
            &self,
            _root: &Path,
            directory: &str,
        ) -> Result<bool, Error> {
            self.asked_under.borrow_mut().push(directory.to_owned());
            Ok(self.git_tracks_the_store)
        }
    }

    fn queries(jj: Result<bool, ()>, git: Result<bool, ()>) -> Queries {
        Queries {
            jj,
            git,
            git_tracks_the_store: false,
            asked_under: RefCell::new(Vec::new()),
        }
    }

    struct Tree {
        _dir: tempfile::TempDir,
        root: PathBuf,
        file: PathBuf,
    }

    type TestError = Box<dyn std::error::Error>;

    fn tree() -> Result<Tree, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("vcs-tracking-unit-")
            .tempdir()?;
        let root = dunce::canonicalize(dir.path())?;
        let file = root.join(".accelerator/config.local.md");
        std::fs::create_dir_all(root.join(".accelerator"))?;
        std::fs::write(&file, "")?;
        Ok(Tree {
            _dir: dir,
            root,
            file,
        })
    }

    #[test]
    fn a_marker_that_cannot_be_probed_is_unknown() -> Result<(), TestError> {
        let tree = tree()?;
        let markers = Markers {
            present: Vec::new(),
            failing: Some(tree.root.join(".git")),
        };

        assert_eq!(
            file_tracking_with(
                &tree.file,
                &markers,
                &queries(Ok(false), Ok(false))
            ),
            FileTracking::Unknown
        );
        Ok(())
    }

    #[test]
    fn no_marker_is_untracked() -> Result<(), TestError> {
        let tree = tree()?;
        let markers = Markers {
            present: Vec::new(),
            failing: None,
        };

        assert_eq!(
            file_tracking_with(
                &tree.file,
                &markers,
                &queries(Ok(true), Ok(true))
            ),
            FileTracking::Untracked
        );
        Ok(())
    }

    #[test]
    fn a_tracked_answer_beats_a_failed_one() -> Result<(), TestError> {
        let tree = tree()?;
        let markers = Markers {
            present: vec![tree.root.join(".jj"), tree.root.join(".git")],
            failing: None,
        };

        assert_eq!(
            file_tracking_with(
                &tree.file,
                &markers,
                &queries(Err(()), Ok(true))
            ),
            FileTracking::Tracked
        );
        assert_eq!(
            file_tracking_with(
                &tree.file,
                &markers,
                &queries(Err(()), Ok(false))
            ),
            FileTracking::Unknown
        );
        Ok(())
    }

    #[test]
    fn a_jj_store_git_tracks_trips_an_untracked_answer_to_unknown(
    ) -> Result<(), TestError> {
        let tree = tree()?;
        let markers = Markers {
            present: vec![
                tree.root.join(".accelerator/.jj"),
                tree.root.join(".git"),
            ],
            failing: None,
        };
        let mut tripped = queries(Ok(false), Ok(false));
        tripped.git_tracks_the_store = true;

        assert_eq!(
            file_tracking_with(&tree.file, &markers, &tripped),
            FileTracking::Unknown
        );
        assert_eq!(*tripped.asked_under.borrow(), [".accelerator/.jj"]);

        let genuine = queries(Ok(false), Ok(false));
        assert_eq!(
            file_tracking_with(&tree.file, &markers, &genuine),
            FileTracking::Untracked
        );
        Ok(())
    }

    #[test]
    fn a_missing_file_is_unknown() -> Result<(), TestError> {
        let tree = tree()?;
        let markers = Markers {
            present: Vec::new(),
            failing: None,
        };

        assert_eq!(
            file_tracking_with(
                &tree.root.join("absent.md"),
                &markers,
                &queries(Ok(false), Ok(false))
            ),
            FileTracking::Unknown
        );
        Ok(())
    }
}
