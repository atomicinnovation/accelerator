//! Answers the consent policy's tracking question through `vcs tracking`.
//!
//! The sub-binary runs as a captured child, so the launcher links no VCS
//! library, and a failure, hang or panic in the VCS code fails closed as
//! `Unknown`.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use config::consent::ConfigFileTracking;
use config::consent::Tracking;
use config::consent::TrackingCheck;
use kernel::TrackingAnswer;

use crate::launch::core::{
    CaptureBinary, Captured, ExternalCommand, ResolveBinary,
};

const DEADLINE: Duration = Duration::from_secs(2);

pub struct DispatchedTracking<R, C> {
    resolver: R,
    capture: C,
}

impl<R: ResolveBinary, C: CaptureBinary> DispatchedTracking<R, C> {
    pub const fn new(resolver: R, capture: C) -> Self {
        Self { resolver, capture }
    }
}

impl<R: ResolveBinary, C: CaptureBinary> ConfigFileTracking
    for DispatchedTracking<R, C>
{
    fn tracking(&self, path: &Path) -> Tracking {
        match self.check(path) {
            TrackingCheck::Known(tracking) => tracking,
            TrackingCheck::Unchecked => Tracking::Unknown,
        }
    }

    fn check(&self, path: &Path) -> TrackingCheck {
        let command = ExternalCommand {
            name: OsString::from("vcs"),
            args: vec![
                OsString::from("tracking"),
                OsString::from("--path"),
                path.as_os_str().to_owned(),
            ],
        };
        let Ok(program) = self.resolver.resolve(&command) else {
            return TrackingCheck::Unchecked;
        };
        let answer = self
            .capture
            .capture(&program, &command.args, DEADLINE)
            .ok()
            .and_then(answered);
        TrackingCheck::Known(match answer {
            Some(TrackingAnswer::Tracked) => Tracking::Tracked,
            Some(TrackingAnswer::Untracked) => Tracking::Untracked,
            None => Tracking::Unknown,
        })
    }
}

fn answered(captured: Captured) -> Option<TrackingAnswer> {
    if !captured.succeeded {
        return None;
    }
    String::from_utf8(captured.stdout).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Duration;

    use config::consent::ConfigFileTracking as _;
    use config::consent::Tracking;
    use config::consent::TrackingCheck;

    use super::DispatchedTracking;
    use crate::launch::core::{
        CaptureBinary, CaptureFailure, Captured, ExternalCommand,
        ResolutionError, ResolveBinary,
    };

    const PERSONAL: &str = "/repo/.accelerator/config.local.md";
    const VCS: &str = "/cache/accelerator-vcs";

    struct Resolves(Option<PathBuf>);

    impl ResolveBinary for Resolves {
        fn resolve(
            &self,
            command: &ExternalCommand,
        ) -> Result<PathBuf, ResolutionError> {
            assert_eq!(command.name, "vcs");
            self.0.clone().ok_or_else(|| ResolutionError::Fetch {
                target: "vcs".to_owned(),
                url: "https://example.test/vcs".to_owned(),
            })
        }
    }

    struct Answers {
        outcome: Result<Captured, CaptureFailure>,
        seen: RefCell<Vec<(PathBuf, Vec<OsString>, Duration)>>,
    }

    impl Answers {
        fn with(outcome: Result<Captured, CaptureFailure>) -> Self {
            Self {
                outcome,
                seen: RefCell::new(Vec::new()),
            }
        }

        fn printing(succeeded: bool, stdout: &str) -> Self {
            Self::with(Ok(Captured {
                succeeded,
                stdout: stdout.as_bytes().to_vec(),
            }))
        }
    }

    impl CaptureBinary for &Answers {
        fn capture(
            &self,
            program: &Path,
            args: &[OsString],
            deadline: Duration,
        ) -> Result<Captured, CaptureFailure> {
            self.seen.borrow_mut().push((
                program.to_path_buf(),
                args.to_vec(),
                deadline,
            ));
            self.outcome.clone()
        }
    }

    fn check(answers: &Answers) -> TrackingCheck {
        DispatchedTracking::new(Resolves(Some(PathBuf::from(VCS))), answers)
            .check(Path::new(PERSONAL))
    }

    #[test]
    fn a_printed_answer_is_known() {
        assert_eq!(
            check(&Answers::printing(true, "tracked\n")),
            TrackingCheck::Known(Tracking::Tracked)
        );
        assert_eq!(
            check(&Answers::printing(true, "untracked\n")),
            TrackingCheck::Known(Tracking::Untracked)
        );
    }

    #[test]
    fn the_resolved_vcs_binary_is_asked_about_the_file_within_two_seconds() {
        let answers = Answers::printing(true, "untracked\n");

        check(&answers);

        assert_eq!(
            answers.seen.into_inner(),
            vec![(
                PathBuf::from(VCS),
                ["tracking", "--path", PERSONAL]
                    .map(OsString::from)
                    .to_vec(),
                Duration::from_secs(2),
            )]
        );
    }

    #[test]
    fn an_unresolvable_vcs_binary_leaves_the_file_unchecked() {
        let answers = Answers::printing(true, "untracked\n");

        let checked = DispatchedTracking::new(Resolves(None), &answers)
            .check(Path::new(PERSONAL));

        assert_eq!(checked, TrackingCheck::Unchecked);
        assert!(answers.seen.into_inner().is_empty());
    }

    #[test]
    fn every_other_outcome_is_unknown() {
        for answers in [
            Answers::printing(false, ""),
            Answers::printing(false, "untracked\n"),
            Answers::printing(true, "maybe\n"),
            Answers::printing(true, ""),
            Answers::with(Err(CaptureFailure::TimedOut)),
            Answers::with(Err(CaptureFailure::OutputExceeded { limit: 4096 })),
            Answers::with(Err(CaptureFailure::CouldNotSpawn {
                program: PathBuf::from(VCS),
                detail: "No such file or directory".to_owned(),
            })),
        ] {
            assert_eq!(
                check(&answers),
                TrackingCheck::Known(Tracking::Unknown),
                "{:?}",
                answers.outcome
            );
        }
    }

    #[test]
    fn a_tracking_question_that_could_not_be_put_fails_closed() {
        let answers = Answers::printing(true, "untracked\n");

        let tracking = DispatchedTracking::new(Resolves(None), &answers)
            .tracking(Path::new(PERSONAL));

        assert_eq!(tracking, Tracking::Unknown);
    }

    #[test]
    fn a_non_utf8_answer_is_unknown() {
        let answers = Answers::with(Ok(Captured {
            succeeded: true,
            stdout: vec![0xff, 0xfe],
        }));

        assert_eq!(check(&answers), TrackingCheck::Known(Tracking::Unknown));
    }
}
