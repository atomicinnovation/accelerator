//! The consent policy's tracking port over the fail-closed VCS walk.

use std::path::Path;

use config::consent::ConfigFileTracking;
use config::consent::Tracking;
use vcs::tracking::FileTracking;

/// Answers whether a config file is tracked by every repository enclosing it.
#[derive(Debug, Clone, Copy, Default)]
pub struct VcsConfigFileTracking;

impl ConfigFileTracking for VcsConfigFileTracking {
    fn tracking(&self, path: &Path) -> Tracking {
        translated(vcs_adapters::file_tracking(path))
    }
}

const fn translated(answer: FileTracking) -> Tracking {
    match answer {
        FileTracking::Untracked => Tracking::Untracked,
        FileTracking::Tracked => Tracking::Tracked,
        FileTracking::Unknown => Tracking::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use config::consent::Tracking;
    use vcs::tracking::FileTracking;

    use super::translated;

    #[test]
    fn each_file_tracking_answer_maps_to_its_tracking() {
        assert_eq!(translated(FileTracking::Untracked), Tracking::Untracked);
        assert_eq!(translated(FileTracking::Tracked), Tracking::Tracked);
        assert_eq!(translated(FileTracking::Unknown), Tracking::Unknown);
    }
}
