//! The consent policy's tracking port over the fail-closed VCS walk.

use std::path::Path;

use config::consent::ConfigFileTracking;
use config::consent::Tracking;
use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;

/// Answers whether a config file is tracked by every repository enclosing it.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrackedConfigFile<T>(pub T);

impl<T: RepositoryTracking> ConfigFileTracking for TrackedConfigFile<T> {
    fn tracking(&self, path: &Path) -> Tracking {
        translated(self.0.file_tracking(path))
    }
}

const fn translated(answer: FileTracking) -> Tracking {
    match answer {
        FileTracking::Untracked => Tracking::Untracked,
        FileTracking::Tracked => Tracking::Tracked,
        FileTracking::Unknown => Tracking::Unknown,
    }
}
