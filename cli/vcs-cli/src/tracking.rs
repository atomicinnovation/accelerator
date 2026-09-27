//! `vcs tracking`: whether the repositories enclosing a file track it, asked
//! so that it fails closed.

use std::path::Path;

use kernel::TrackingAnswer;
use vcs::tracking::FileTracking;

/// # Errors
///
/// When a repository enclosing `path` could not say whether it tracks it.
pub fn run(
    path: &Path,
    file_tracking: impl Fn(&Path) -> FileTracking,
) -> Result<TrackingAnswer, kernel::Error> {
    match file_tracking(path) {
        FileTracking::Tracked => Ok(TrackingAnswer::Tracked),
        FileTracking::Untracked => Ok(TrackingAnswer::Untracked),
        FileTracking::Unknown => Err(kernel::Error::Failed(format!(
            "could not determine whether {} is tracked",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use kernel::TrackingAnswer;
    use vcs::tracking::FileTracking;

    use super::run;

    #[test]
    fn a_known_answer_is_printable() -> Result<(), kernel::Error> {
        let path = Path::new("/repo/.accelerator/config.local.md");

        assert_eq!(
            run(path, |_| FileTracking::Tracked)?,
            TrackingAnswer::Tracked
        );
        assert_eq!(
            run(path, |_| FileTracking::Untracked)?,
            TrackingAnswer::Untracked
        );
        Ok(())
    }

    #[test]
    fn an_unknown_answer_is_a_failure_naming_the_file() {
        let path = Path::new("/repo/.accelerator/config.local.md");

        let error = run(path, |_| FileTracking::Unknown)
            .err()
            .map(|error| error.to_string());

        assert!(error
            .is_some_and(|message| message
                .contains("/repo/.accelerator/config.local.md")));
    }
}
