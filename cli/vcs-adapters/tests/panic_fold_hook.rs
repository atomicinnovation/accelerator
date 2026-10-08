//! The panic hook a fold leaves in place, in a process of its own: a hook is
//! process-global, so a sibling test could otherwise install or observe it.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::panic;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;
use vcs_adapters::PanicFold;

struct Panicking;

impl RepositoryTracking for Panicking {
    fn file_tracking(&self, _path: &Path) -> FileTracking {
        panic!("index exploded")
    }

    fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
        panic!("workspace exploded")
    }
}

#[test]
fn only_a_panic_outside_a_fold_reaches_the_installed_hook() {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let recording = Arc::clone(&seen);
    panic::set_hook(Box::new(move |info| {
        recording.lock().unwrap().push(info.to_string());
    }));

    let folded = PanicFold(Panicking).file_tracking(Path::new("/repo/file"));
    let unfolded = panic::catch_unwind(|| panic!("launcher exploded"));

    drop(panic::take_hook());
    let reported = seen.lock().unwrap().clone();
    assert_eq!(folded, FileTracking::Unknown);
    assert!(unfolded.is_err());
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(reported[0].contains("launcher exploded"), "{reported:?}");
}
