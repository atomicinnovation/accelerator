//! The consent policy's wiring over the VCS tracking port: what "inside the
//! repository" means, and whether a config file is tracked.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use config::consent::ConfigFileTracking;
use config::consent::Tracking;
use config_adapters::credential_ports;
use config_adapters::repository_roots;
use config_adapters::TrackedConfigFile;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;

struct Answering {
    file: FileTracking,
    roots: RootsAnswer,
}

impl Answering {
    const fn outside_any_repository() -> Self {
        Self {
            file: FileTracking::Untracked,
            roots: RootsAnswer {
                roots: Vec::new(),
                complete: true,
            },
        }
    }

    fn tracking(file: FileTracking) -> Self {
        Self {
            file,
            ..Self::outside_any_repository()
        }
    }
}

impl RepositoryTracking for Answering {
    fn file_tracking(&self, _path: &Path) -> FileTracking {
        self.file
    }

    fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
        self.roots.clone()
    }
}

#[derive(Clone, Default)]
struct Recording {
    asked: Rc<RefCell<Vec<PathBuf>>>,
}

impl RepositoryTracking for Recording {
    fn file_tracking(&self, path: &Path) -> FileTracking {
        self.asked.borrow_mut().push(path.to_path_buf());
        FileTracking::Tracked
    }

    fn repository_roots(&self, directory: &Path) -> RootsAnswer {
        self.asked.borrow_mut().push(directory.to_path_buf());
        RootsAnswer {
            roots: Vec::new(),
            complete: true,
        }
    }
}

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let scratch = tempfile::Builder::new()
        .prefix("consent-roots-")
        .tempdir()
        .unwrap();
    let base = scratch.path().canonicalize().unwrap();
    (scratch, base)
}

#[test]
fn outside_any_repository_the_config_root_alone_is_inside() {
    let (_scratch, base) = scratch();
    let config_root = base.join("project");
    let cwd = base.join("elsewhere");
    std::fs::create_dir_all(&config_root).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();

    let roots = repository_roots(
        &Answering::outside_any_repository(),
        &config_root,
        &cwd,
    );

    assert!(roots.is_complete());
    assert!(roots.contains(&config_root.join("bin")));
    assert!(!roots.contains(&cwd));
    assert!(!roots.contains(Path::new("/usr/bin")));
}

#[test]
fn a_config_root_that_cannot_be_resolved_leaves_the_roots_incomplete() {
    let (_scratch, base) = scratch();

    let roots = repository_roots(
        &Answering::outside_any_repository(),
        &base.join("missing"),
        &base,
    );

    assert!(!roots.is_complete());
}

#[test]
fn each_enclosing_repository_answer_keeps_its_completeness() {
    let (_scratch, base) = scratch();
    for complete in [true, false] {
        let tracking = Answering {
            roots: RootsAnswer {
                roots: vec![PathBuf::from("/work/repo")],
                complete,
            },
            ..Answering::outside_any_repository()
        };

        let roots = repository_roots(&tracking, &base, &base);

        assert_eq!(roots.is_complete(), complete);
        assert!(roots.contains(Path::new("/work/repo/bin")));
        assert!(roots.contains(&base.join("bin")));
    }
}

#[test]
fn a_config_file_is_tracked_as_the_repositories_answer() {
    for (answer, expected) in [
        (FileTracking::Untracked, Tracking::Untracked),
        (FileTracking::Tracked, Tracking::Tracked),
        (FileTracking::Unknown, Tracking::Unknown),
    ] {
        let tracked = TrackedConfigFile(Answering::tracking(answer));

        assert_eq!(tracked.tracking(Path::new("config.local.md")), expected);
    }
}

#[test]
fn the_credential_ports_ask_the_given_tracking_about_the_config_file() {
    let (_scratch, base) = scratch();
    let tracking = Recording::default();
    let asked = Rc::clone(&tracking.asked);
    let ports = credential_ports(tracking, &base, &base.join("cwd"));
    let personal = base.join(".accelerator/config.local.md");

    let answer = ports.tracking.tracking(&personal);

    assert_eq!(answer, Tracking::Tracked);
    assert_eq!(*asked.borrow(), [base.join("cwd"), personal]);
}
