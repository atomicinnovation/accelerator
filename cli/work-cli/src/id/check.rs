//! Adapter/binary wiring for `work id check`: settles the ID a new item
//! would take, writing nothing and taking no create lock.

use std::path::Path;

use ::config::ConfigAccess;
use work::id_pattern::IdKey;
use work::id_scheme::IdScheme;
use work::requested_id::Proposal;
use work::requested_id::RequestedId;

use crate::config::locate_work_items;
use crate::identity_run::IdentityRunStart;
use crate::requested_id::refuse_if_requested_id_taken;

pub enum RunOutcome {
    Checked(String),
    Failed(String),
    /// The identity run could not begin; nothing was read.
    Refused {
        message: String,
        code: u8,
    },
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    requested: &str,
    identity_run: &IdentityRunStart<'_>,
) -> RunOutcome {
    let location = match locate_work_items(start, config) {
        Ok(location) => location,
        Err(message) => return RunOutcome::Failed(message),
    };
    let _identity_run = match identity_run(&location.root, "id check") {
        Ok(guard) => guard,
        Err(refusal) => {
            let (message, code) =
                crate::identity_run::refused(refusal, &location.root);
            return RunOutcome::Refused { message, code };
        }
    };
    let proposal = match RequestedId::propose(requested, &location.scheme) {
        Ok(proposal) => proposal,
        Err(refusal) => return RunOutcome::Failed(refusal.to_string()),
    };
    let id = match &proposal {
        Proposal::Exact(id)
        | Proposal::OtherKey(id)
        | Proposal::Corrected { id, .. } => id,
    };
    match refuse_if_requested_id_taken(id, &location) {
        Ok(()) => RunOutcome::Checked(verdict(&proposal, &location.scheme)),
        Err(refusal) => RunOutcome::Failed(refusal),
    }
}

fn verdict(proposal: &Proposal, scheme: &IdScheme) -> String {
    let configured_key = || scheme.key().map(IdKey::as_str).unwrap_or_default();
    match proposal {
        Proposal::Exact(id) => format!("exact\t{}", id.as_str()),
        Proposal::OtherKey(id) => {
            format!("other-key\t{}\t{}", id.as_str(), configured_key())
        }
        Proposal::Corrected {
            id,
            key_differs: false,
        } => format!("corrected\t{}", id.as_str()),
        Proposal::Corrected {
            id,
            key_differs: true,
        } => format!("corrected\t{}\t{}", id.as_str(), configured_key()),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use std::rc::Rc;

    use tracker_test_support::RecordingTracker;

    use super::run;
    use super::RunOutcome;
    use crate::create::CreateArgs;
    use crate::test_support::meta_tree;
    use crate::test_support::PushingRepo;

    type RepoFor = fn() -> PushingRepo;

    fn check(repo: &PushingRepo, id: &str) -> RunOutcome {
        run(
            repo.root.path(),
            &repo.config,
            id,
            &crate::identity_run::begin,
        )
    }

    fn checked(repo: &PushingRepo, id: &str) -> String {
        match check(repo, id) {
            RunOutcome::Checked(verdict) => verdict,
            RunOutcome::Failed(message)
            | RunOutcome::Refused { message, .. } => {
                panic!("{id} was refused: {message}")
            }
        }
    }

    fn failed(repo: &PushingRepo, id: &str) -> String {
        match check(repo, id) {
            RunOutcome::Failed(message) => message,
            RunOutcome::Checked(verdict) => {
                panic!("{id} was checked: {verdict}")
            }
            RunOutcome::Refused { message, .. } => {
                panic!("{id} was refused before checking: {message}")
            }
        }
    }

    fn create_refusal(repo: &PushingRepo, id: &str) -> String {
        let args = CreateArgs {
            dry_run: true,
            ..PushingRepo::args_with_id(id, false)
        };
        let tracker = Rc::new(RecordingTracker::holding(Vec::new()));
        match repo.run_args(&args, &tracker, None) {
            crate::create::RunOutcome::Failed(message) => message,
            _ => panic!("work create --id {id} was not refused"),
        }
    }

    fn code_of(message: &str) -> &str {
        message.split(':').next().unwrap_or_default()
    }

    fn numeric() -> PushingRepo {
        PushingRepo::new()
    }

    fn holding_0400() -> PushingRepo {
        let repo = PushingRepo::new();
        repo.holding("meta/work/0400-x.md", "");
        repo
    }

    #[test]
    fn an_exact_id_prints_exact() {
        assert_eq!(checked(&numeric(), "0400"), "exact\t0400");
        assert_eq!(checked(&PushingRepo::keyed(), "pp-0400"), "exact\tpp-0400");
    }

    #[test]
    fn an_exact_id_under_another_key_prints_other_key() {
        let project = PushingRepo::configured(&[
            ("work.id_pattern", "{project}-{number:04d}"),
            ("work.default_project_code", "PP"),
        ]);

        assert_eq!(
            checked(&PushingRepo::keyed(), "XY-0400"),
            "other-key\tXY-0400\tPP"
        );
        assert_eq!(checked(&project, "XY-0400"), "other-key\tXY-0400\tPP");
    }

    #[test]
    fn a_correctable_id_prints_its_correction() {
        assert_eq!(checked(&numeric(), "400"), "corrected\t0400");
        assert_eq!(checked(&PushingRepo::keyed(), "400"), "corrected\tPP-0400");
    }

    #[test]
    fn a_correction_under_another_key_names_the_configured_key() {
        let keyed = PushingRepo::keyed();

        assert_eq!(checked(&keyed, "PO-400"), "corrected\tPO-0400\tPP");
        assert_eq!(checked(&keyed, "xy-400"), "corrected\txy-0400\tPP");
    }

    #[test]
    fn each_refusal_carries_the_code_work_create_gives() {
        let cases: [(RepoFor, &str, &str); 5] = [
            (PushingRepo::tracker_owned, "0400", "E_ID_TRACKER_OWNED"),
            (numeric, "draft-k7mq3x", "E_ID_DRAFT_RESERVED"),
            (numeric, "XY-400", "E_ID_PATTERN_MISMATCH"),
            (numeric, "0", "E_ID_PATTERN_MISMATCH"),
            (holding_0400, "0400", "E_ID_TAKEN"),
        ];
        for (repo_for, id, code) in cases {
            let repo = repo_for();

            let message = failed(&repo, id);

            assert_eq!(code_of(&message), code, "{id}: {message}");
            assert_eq!(code_of(&create_refusal(&repo, id)), code, "{id}");
        }
    }

    #[test]
    fn a_corrected_id_that_is_taken_is_refused_naming_its_holder() {
        let message = failed(&holding_0400(), "400");

        assert!(message.starts_with("E_ID_TAKEN: "), "{message}");
        assert!(message.contains("0400"), "{message}");
        assert!(message.contains("0400-x.md"), "{message}");
    }

    #[test]
    fn a_check_writes_nothing() {
        let cases: [(RepoFor, &str); 9] = [
            (numeric, "0400"),
            (numeric, "400"),
            (PushingRepo::keyed, "XY-0400"),
            (PushingRepo::keyed, "PO-400"),
            (PushingRepo::tracker_owned, "0400"),
            (numeric, "draft-k7mq3x"),
            (numeric, "XY-400"),
            (numeric, "0"),
            (holding_0400, "400"),
        ];
        for (repo_for, id) in cases {
            let repo = repo_for();
            let had_work_dir = repo.path("meta/work").exists();
            let before = meta_tree(repo.root.path());

            let _ = check(&repo, id);

            assert_eq!(meta_tree(repo.root.path()), before, "{id}");
            assert_eq!(repo.path("meta/work").exists(), had_work_dir, "{id}");
        }
    }

    #[test]
    fn a_check_does_not_wait_for_the_create_lock() {
        use corpus_adapters::LockdirLock;

        let cases = [
            (Some("meta/work/0400-other.md"), "E_ID_TAKEN: "),
            (None, "exact\t0400"),
        ];
        for (holder, expected) in cases {
            let repo = std::sync::Arc::new(PushingRepo::new());
            std::fs::create_dir_all(repo.path("meta/work")).expect("work dir");
            if let Some(holder) = holder {
                repo.holding(holder, "");
            }
            let create_lock = store::lock::acquire(
                &repo.path("meta/work").join(LockdirLock::CREATE_LOCKDIR),
                store::lock::LockOptions::default(),
            )
            .expect("the create lock is free");
            let (done, finished) = std::sync::mpsc::channel();
            let checking = std::sync::Arc::clone(&repo);

            let checker = std::thread::spawn(move || {
                let outcome = match check(&checking, "0400") {
                    RunOutcome::Checked(verdict) => verdict,
                    RunOutcome::Failed(message)
                    | RunOutcome::Refused { message, .. } => message,
                };
                let _ = done.send(outcome);
            });

            let outcome =
                finished.recv_timeout(std::time::Duration::from_secs(10));
            drop(create_lock);
            checker.join().expect("joined");
            let outcome = outcome.expect("the check waited on the lock");
            assert!(outcome.starts_with(expected), "{outcome}");
        }
    }

    #[test]
    fn a_rekey_in_progress_refuses_the_check() {
        let repo = numeric();
        crate::test_support::rekey_in_progress(repo.root.path());

        let RunOutcome::Refused { message, code } = check(&repo, "0400") else {
            panic!("the check ran during a re-key");
        };

        assert_eq!(code, crate::exit_codes::UNRESOLVED);
        assert!(
            message.starts_with("Error: E_REKEY_IN_PROGRESS"),
            "{message}"
        );
    }

    #[test]
    fn a_config_error_refuses_the_check() {
        let keyless = PushingRepo::configured(&[(
            "work.id_pattern",
            "{key}-{number:04d}",
        )]);
        let malformed =
            PushingRepo::configured(&[("work.id_pattern", "{number:4d}")]);

        assert!(
            failed(&keyless, "XY-0400").starts_with("E_WORK_KEY_REQUIRED"),
            "{}",
            failed(&keyless, "XY-0400")
        );
        assert_eq!(
            failed(&malformed, "0400"),
            create_refusal(&malformed, "0400")
        );
        assert!(
            failed(&malformed, "0400").starts_with("E_WORK_ID_PATTERN_INVALID"),
            "{}",
            failed(&malformed, "0400")
        );
    }
}
