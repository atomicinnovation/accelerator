//! The two stores a sync run writes and removes its files through, each
//! confined to one validated root.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::store::FileRemove;
use corpus::store::StoreError;
use corpus_adapters::FileCorpusStore;
use work_adapters::sync::files::SyncStore;

use crate::bounded_roots::BoundedRoot;

/// Builds a run's two stores; production builds each over its root's bound.
pub struct StoreFactories<'a> {
    pub work_items: &'a dyn Fn(&BoundedRoot) -> Box<dyn SyncStore>,
    pub integration_state: &'a dyn Fn(&BoundedRoot) -> Box<dyn SyncStore>,
}

impl StoreFactories<'static> {
    pub const fn production() -> Self {
        Self {
            work_items: &rooted_store,
            integration_state: &rooted_store,
        }
    }
}

pub struct SyncStores {
    pub work_items: Box<dyn SyncStore>,
    pub integration_state: Box<dyn SyncStore>,
}

impl SyncStores {
    pub fn bounded(
        work_items: &BoundedRoot,
        integration_state: &BoundedRoot,
        factories: &StoreFactories<'_>,
    ) -> Self {
        Self {
            work_items: (factories.work_items)(work_items),
            integration_state: (factories.integration_state)(integration_state),
        }
    }
}

/// A store over `root` that also refuses any path naming `.` or `..`
/// below it, even one resolving inside: the engine builds every path it
/// writes from the normalised root, so such a path was never its own.
pub fn rooted_store(root: &BoundedRoot) -> Box<dyn SyncStore> {
    Box::new(RootedStore {
        root: root.root().to_path_buf(),
        inner: FileCorpusStore::within_project(root.root(), root.bound()),
    })
}

struct RootedStore {
    root: PathBuf,
    inner: FileCorpusStore,
}

impl RootedStore {
    fn admit(&self, path: &Path) -> Result<(), StoreError> {
        let plain_below_root =
            path.strip_prefix(&self.root).is_ok_and(|below| {
                below.components().next().is_some()
                    && below.components().all(|component| {
                        matches!(component, Component::Normal(_))
                    })
            });
        if plain_below_root {
            Ok(())
        } else {
            Err(StoreError::UnsafePath {
                path: path.display().to_string(),
            })
        }
    }
}

impl AtomicWrite for RootedStore {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.admit(path)?;
        self.inner.write(path, bytes)
    }
}

impl FileRemove for RootedStore {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.admit(path)?;
        self.inner.remove(path)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use ::config::provenance::PathSource;
    use corpus::store::StoreError;

    use super::rooted_store;
    use crate::bounded_roots::BoundedRoot;
    use crate::bounded_roots::ConfiguredRoot;
    use crate::bounded_roots::RootKey;

    struct Repository {
        dir: tempfile::TempDir,
    }

    impl Repository {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir_all(dir.path().join("meta/work"))
                .expect("mkdir");
            std::fs::create_dir_all(
                dir.path().join(".accelerator/state/integrations/jira"),
            )
            .expect("mkdir");
            Self { dir }
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        fn bounded(&self, key: RootKey, relative: &str) -> BoundedRoot {
            BoundedRoot::for_configured(
                &ConfiguredRoot {
                    key,
                    path: self.path(relative),
                    source: PathSource::Default,
                },
                self.dir.path(),
            )
            .expect("an in-repository root")
        }

        fn work_items(&self) -> BoundedRoot {
            self.bounded(RootKey::Work, "meta/work")
        }

        fn integration_state(&self) -> BoundedRoot {
            self.bounded(
                RootKey::Integrations,
                ".accelerator/state/integrations",
            )
            .child("jira")
        }
    }

    fn refused(result: &Result<(), StoreError>, path: &Path) {
        assert!(
            matches!(result, Err(StoreError::UnsafePath { .. })),
            "{} was not refused",
            path.display()
        );
        assert!(!path.exists(), "{} was written", path.display());
    }

    #[test]
    fn the_work_item_store_refuses_integration_state() {
        let repository = Repository::new();
        let store = rooted_store(&repository.work_items());
        let baseline = repository
            .path(".accelerator/state/integrations/jira/last-sync.json");

        refused(&store.write(&baseline, b"{}"), &baseline);
    }

    #[test]
    fn the_work_item_store_refuses_a_dotdot_escape() {
        let repository = Repository::new();
        let store = rooted_store(&repository.work_items());

        for escape in [
            "meta/work/../escaped.md",
            "meta/work/missing/../../escaped.md",
        ] {
            let path = repository.path(escape);
            refused(
                &store.write(&path, b"x"),
                &repository.path("meta/escaped.md"),
            );
        }
    }

    #[test]
    fn the_integration_state_store_refuses_the_work_directory() {
        let repository = Repository::new();
        let store = rooted_store(&repository.integration_state());
        let item = repository.path("meta/work/0001-x.md");
        std::fs::write(&item, "mine").expect("seed");

        assert!(matches!(
            store.write(&item, b"theirs"),
            Err(StoreError::UnsafePath { .. })
        ));
        assert!(matches!(
            store.remove(&item),
            Err(StoreError::UnsafePath { .. })
        ));
        assert_eq!(std::fs::read_to_string(&item).expect("read"), "mine");
    }

    #[test]
    fn the_integration_state_store_refuses_a_dotdot_escape() {
        let repository = Repository::new();
        let store = rooted_store(&repository.integration_state());
        let root = ".accelerator/state/integrations/jira";

        for escape in ["../escaped.json", "missing/../../escaped.json"] {
            let path = repository.path(&format!("{root}/{escape}"));
            refused(
                &store.write(&path, b"x"),
                &repository
                    .path(".accelerator/state/integrations/escaped.json"),
            );
        }
    }

    #[test]
    fn the_integration_state_store_refuses_a_dotdot_that_stays_inside() {
        let repository = Repository::new();
        let store = rooted_store(&repository.integration_state());
        let baseline = repository
            .path(".accelerator/state/integrations/jira/last-sync.json");
        let through_pending_push = repository.path(
            ".accelerator/state/integrations/jira/pending-push/../last-sync.json",
        );

        refused(&store.write(&through_pending_push, b"{}"), &baseline);
    }

    #[test]
    fn a_work_directory_symlinked_out_of_the_repository_is_refused() {
        let repository = Repository::new();
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let store = rooted_store(&repository.work_items());
        let work = repository.path("meta/work");
        std::fs::remove_dir(&work).expect("rmdir");
        std::os::unix::fs::symlink(elsewhere.path(), &work).expect("symlink");

        let item = work.join("0001-x.md");

        refused(
            &store.write(&item, b"x"),
            &elsewhere.path().join("0001-x.md"),
        );
    }
}
