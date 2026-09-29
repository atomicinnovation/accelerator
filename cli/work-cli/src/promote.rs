//! `accelerator work promote`: promotes one draft through the service
//! `work sync` runs for every draft, with the `--adopt` and `--create`
//! recoveries for a draft whose earlier create may have reached the tracker.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus_adapters::FileCorpusStore;
use tracker::ExternalId;
use vcs_adapters::library::InProcessProbe;
use work::draft_id::DraftId;
use work::identity::resolve_identity;
use work::identity::IdentityMatch;
use work::identity::IdentityResolution;
use work::promotion::NotPromoted;
use work::promotion::PromotionMode;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles as _;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::promotion::promote;
use work_adapters::promotion::PromotionOutcome;
use work_adapters::promotion::PromotionPorts;
use work_adapters::promotion::PromotionRow;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::identity_settlement::finish_interrupted_retirements;
use work_adapters::sync::identity_settlement::IdentityRow;
use work_adapters::sync::identity_settlement::SettlementPorts;
use work_adapters::sync::run::RunError;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::cli::PromoteArgs;
use crate::exit_codes;
use crate::identity_workspace::IdentityWorkspace;
use crate::promotion_report::detail_lines;
use crate::promotion_report::promotion_exit;
use crate::promotion_report::promotion_line;
use crate::sync::severity;
use crate::sync::standalone_identity_line;
use crate::tracker_registry::TrackerRegistry;

pub const E_PROMOTE_NOT_A_DRAFT: &str = "E_PROMOTE_NOT_A_DRAFT";
pub const E_PROMOTE_NOT_TRACKER_OWNED: &str = "E_PROMOTE_NOT_TRACKER_OWNED";
pub const E_PROMOTE_CONFLICTING: &str = "E_PROMOTE_CONFLICTING";

#[derive(Debug)]
pub enum PromoteOutcome {
    /// The rows to print, what stderr tells the user beside them, and the
    /// code to exit with.
    Reported {
        lines: Vec<String>,
        remedy: Option<String>,
        code: u8,
    },
    NotADraft(String),
    Failed {
        message: String,
        code: u8,
    },
}

fn failed(code: u8) -> impl Fn(String) -> PromoteOutcome {
    move |message| PromoteOutcome::Failed { message, code }
}

fn not_a_draft(input: &str) -> PromoteOutcome {
    PromoteOutcome::NotADraft(format!(
        "{E_PROMOTE_NOT_A_DRAFT}: '{input}' names no draft; `work promote` \
         takes the draft- ID of an item in the drafts directory"
    ))
}

fn not_tracker_owned() -> PromoteOutcome {
    failed(exit_codes::USAGE)(format!(
        "{E_PROMOTE_NOT_TRACKER_OWNED}: `work promote` retires a draft to the \
         key the tracker gives it, which needs work.id_pattern \"{{tracker}}\""
    ))
}

fn conflicting(input: &str, claims: &[IdentityMatch<'_>]) -> PromoteOutcome {
    let claimants = claims.iter().fold(String::new(), |listing, claim| {
        format!(
            "{listing}\n  {} [{}]",
            claim.item.path.display(),
            claim.field.frontmatter_key()
        )
    });
    failed(exit_codes::USAGE)(format!(
        "{E_PROMOTE_CONFLICTING}: '{input}' is claimed by more than one \
         item; resolve the duplicate before promoting:{claimants}"
    ))
}

fn mode_of(args: &PromoteArgs) -> PromotionMode {
    match (&args.adopt, args.create) {
        (Some(key), _) => PromotionMode::Adopt(ExternalId::new(key.clone())),
        (None, true) => PromotionMode::CreateAcceptingDuplicate,
        (None, false) => PromotionMode::Standard,
    }
}

/// What stderr tells the user when a local item stands in the way of the
/// key the draft was to take.
fn remedy_for(
    row: &PromotionRow,
    mode: &PromotionMode,
    draft: &DraftId,
) -> Option<String> {
    let PromotionOutcome::NotPromoted {
        reason: NotPromoted::Refused(refusal),
        held_key,
    } = &row.outcome
    else {
        return None;
    };
    let named = match mode {
        PromotionMode::Adopt(key) => Some(key),
        PromotionMode::Standard | PromotionMode::CreateAcceptingDuplicate => {
            None
        }
    };
    held_key
        .as_ref()
        .or(named)
        .map(|key| crate::create::blocked_remedy(refusal, key, draft))
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through the outcome.
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &PromoteArgs,
    registry: &dyn TrackerRegistry,
) -> PromoteOutcome {
    match try_run(start, config, args, registry) {
        Ok(outcome) | Err(outcome) => outcome,
    }
}

/// The draft `input` names and where it is, once the repository's pattern
/// lets the tracker own IDs.
fn tracker_owned_draft(
    config: &dyn ConfigAccess,
    work_dir: &Path,
    input: &str,
) -> Result<(DraftId, PathBuf), PromoteOutcome> {
    let internal = failed(exit_codes::ERROR);
    let draft = DraftId::parse(input).ok_or_else(|| not_a_draft(input))?;
    let ownership = crate::config::resolve_scheme(config)
        .map_err(|error| internal(error.to_string()))?
        .ownership();
    if ownership != corpus::IdOwnership::Tracker {
        return Err(not_tracker_owned());
    }
    let files = FilesystemWorkItemFiles::new(work_dir)
        .files()
        .map_err(|error| internal(error.to_string()))?;
    let items = identities(&files);
    match resolve_identity(draft.as_str(), &items) {
        IdentityResolution::Unique(item) => Ok((draft, item.path.clone())),
        IdentityResolution::Conflicting(claims) => {
            Err(conflicting(input, &claims))
        }
        IdentityResolution::Unmatched => Err(not_a_draft(input)),
    }
}

fn try_run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &PromoteArgs,
    registry: &dyn TrackerRegistry,
) -> Result<PromoteOutcome, PromoteOutcome> {
    let internal = failed(exit_codes::ERROR);
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir = crate::config::resolve_work_dir(config, &root)
        .map_err(|error| internal(error.to_string()))?;
    let (draft, draft_path) =
        tracker_owned_draft(config, &work_dir, &args.draft_id)?;
    let integration =
        crate::config::effective_nonempty(config, "work.integration")
            .map_err(|error| internal(error.to_string()))?;
    let integrations_root = crate::sync::integrations_dir(config, &root)
        .map_err(|error| internal(error.to_string()))?;
    let tracker = registry.resolve(&integration).map_err(|error| {
        failed(crate::create::dispatch_code_for_selection_error(&error))(
            error.message(),
        )
    })?;
    let workspace = IdentityWorkspace::open(
        config,
        &root,
        &work_dir,
        &integrations_root,
        &integration,
    )
    .map_err(&internal)?;

    let baseline_path = workspace.baseline_path();
    let baseline_dir = baseline_path.parent().unwrap_or(&integrations_root);
    let baseline_store = FileCorpusStore::new(baseline_dir);
    let project_store = FileCorpusStore::new(&root);
    let state_store = FileCorpusStore::new(workspace.state_dir());
    let integrations_store = FileCorpusStore::new(&integrations_root);
    let baseline = workspace.baseline(&baseline_store);
    let retirement = workspace.retirement_ports(&project_store, &baseline);
    let records = workspace.retirement_records(&state_store);
    let promotions = workspace.promotion_records(&integrations_store);
    for warning in
        crate::sync::unreadable_promotion_record_warnings(&promotions)
    {
        eprintln!("{warning}");
    }
    let probe_status = || -> Box<dyn WorkingCopyStatus> {
        Box::new(VcsWorkingCopyStatus::probed_from(&root, &InProcessProbe))
    };
    let settlement = SettlementPorts {
        retirement: &retirement,
        records: &records,
        promotions: &promotions,
        ownership: corpus::IdOwnership::Tracker,
        state_dir: workspace.state_dir(),
        probe_status: &probe_status,
    };
    let resumed = finish_interrupted_retirements(&settlement).map_err(
        |error| match error {
            RunError::RetirementIncomplete { message } => {
                failed(exit_codes::TERMINAL)(message)
            }
            other => internal(format!("{other:?}")),
        },
    )?;

    let mode = mode_of(args);
    let ports = PromotionPorts {
        tracker: tracker.as_ref(),
        retirement: &retirement,
        records: &promotions,
    };
    let result = promote(&draft, &mode, &ports);
    let row = PromotionRow::of(
        &draft,
        draft_path,
        result,
        &ports,
        workspace.state_dir(),
    );
    let awaiting_human = resumed
        .iter()
        .any(IdentityRow::awaits_human)
        .then_some(exit_codes::UNRESOLVED);
    let code = std::iter::once(promotion_exit(&row))
        .chain(awaiting_human)
        .max_by_key(|&code| severity(code))
        .unwrap_or(exit_codes::CLEAN);
    let mut lines: Vec<String> =
        resumed.iter().map(standalone_identity_line).collect();
    lines.push(promotion_line(&row));
    lines.extend(detail_lines(&row));
    Ok(PromoteOutcome::Reported {
        remedy: remedy_for(&row, &mode, &draft),
        lines,
        code,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;
    use std::rc::Rc;

    use corpus_adapters::FileCorpusStore;
    use tracker::ExternalId;
    use tracker::RemoteIssue;
    use tracker::RemoteTimestamp;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;
    use work::draft_id::DraftId;
    use work::promotion::PromotionRecord;
    use work::promotion::PromotionStage;
    use work::retirement::Retirement;
    use work::retirement::RetirementRecord;
    use work::sync::RequestFingerprint;
    use work_adapters::promotion_records::FilePromotionRecords;
    use work_adapters::promotion_records::PromotionRecords as _;
    use work_adapters::retirement_records::FileRetirementRecords;
    use work_adapters::retirement_records::RetirementRecords as _;
    use work_adapters::sync::pending_push;

    use super::PromoteOutcome;
    use crate::cli::PromoteArgs;
    use crate::cli::SyncArgs;
    use crate::exit_codes;
    use crate::finaliser::NoFinaliser;
    use crate::test_support::StubRegistry;

    const DRAFT: &str = "draft-aaaaaa";
    const DRAFT_FILE: &str = "meta/work/drafts/draft-aaaaaa-title.md";
    const INTEGRATIONS: &str = ".accelerator/state/integrations";

    struct Repo {
        dir: tempfile::TempDir,
    }

    impl Repo {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            let git = |args: &[&str]| {
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .status()
                    .expect("git invocation");
            };
            git(&["init", "-q"]);
            git(&["config", "user.name", "Test User"]);
            git(&["config", "user.email", "test@example.com"]);
            let repo = Self { dir };
            repo.write(
                ".accelerator/config.md",
                "---\nwork:\n  integration: jira\n  id_pattern: \
                 \"{tracker}\"\n---\n",
            );
            repo.write(
                DRAFT_FILE,
                "---\nid: \"draft-aaaaaa\"\ntitle: \"Title\"\nkind: \
                 \"task\"\n---\n\n# draft-aaaaaa: Title\n",
            );
            repo
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root().join(relative)
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.path(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        fn read(&self, relative: &str) -> Option<String> {
            std::fs::read_to_string(self.path(relative)).ok()
        }

        fn record_path(&self) -> PathBuf {
            pending_push::record_path(
                &self.path(INTEGRATIONS),
                "jira",
                &draft(),
            )
        }

        fn save_attempted(&self) {
            let store = FileCorpusStore::new(self.root());
            FilePromotionRecords::new(&self.path(INTEGRATIONS), "jira", &store)
                .save(&PromotionRecord {
                    draft_id: draft(),
                    request: RequestFingerprint {
                        title: "Title".to_owned(),
                        digest: "request".to_owned(),
                        attempted_at: 1,
                        failure: Some("response lost".to_owned()),
                    },
                    content_digest: "content".to_owned(),
                    stage: PromotionStage::Attempted,
                })
                .unwrap();
        }

        fn promote(
            &self,
            tracker: &Rc<RecordingTracker>,
            adopt: Option<&str>,
            create: bool,
        ) -> PromoteOutcome {
            self.promote_named(DRAFT, tracker, adopt, create)
        }

        fn promote_named(
            &self,
            draft_id: &str,
            tracker: &Rc<RecordingTracker>,
            adopt: Option<&str>,
            create: bool,
        ) -> PromoteOutcome {
            let composed = config_adapters::compose(
                self.root(),
                config_adapters::LegacyPolicy::Reject,
            )
            .expect("compose the test config");
            super::run(
                self.root(),
                &composed.service,
                &PromoteArgs {
                    draft_id: draft_id.to_owned(),
                    adopt: adopt.map(str::to_owned),
                    create,
                },
                &StubRegistry(Rc::clone(tracker)),
            )
        }
    }

    fn draft() -> DraftId {
        DraftId::parse(DRAFT).unwrap()
    }

    fn held(raw: &str) -> Rc<RecordingTracker> {
        let key = ExternalId::new(raw.to_owned());
        Rc::new(RecordingTracker::holding(vec![(
            key.clone(),
            RemoteIssue {
                key,
                updated: RemoteTimestamp::Reported("t0".to_owned()),
                body: "Title\nWritten in the tracker.\n".to_owned(),
            },
        )]))
    }

    fn fresh() -> Rc<RecordingTracker> {
        Rc::new(RecordingTracker::holding(Vec::new()))
    }

    fn creates(tracker: &RecordingTracker) -> usize {
        tracker
            .calls()
            .iter()
            .filter(|call| matches!(call, Call::Create { .. }))
            .count()
    }

    fn reported(outcome: PromoteOutcome) -> (Vec<String>, Option<String>, u8) {
        match outcome {
            PromoteOutcome::Reported {
                lines,
                remedy,
                code,
            } => (lines, remedy, code),
            other => panic!("not reported: {other:?}"),
        }
    }

    #[test]
    fn promote_one_draft_matches_a_sync_promotion() {
        let promoted = Repo::new();
        let synced = Repo::new();

        let (lines, _, code) =
            reported(promoted.promote(&fresh(), None, false));
        let composed = config_adapters::compose(
            synced.root(),
            config_adapters::LegacyPolicy::Reject,
        )
        .unwrap();
        let synced_code = crate::sync::run_sync(
            synced.root(),
            &composed.service,
            &SyncArgs {
                push_only: false,
                pull_only: false,
                preview: false,
                resolutions: Vec::new(),
                per_item_reads: false,
                max_pulls: None,
                max_pushes: None,
                allow_unbounded: false,
                no_promote: false,
                targets: Vec::new(),
            },
            &StubRegistry(fresh()),
            &NoFinaliser,
        );

        assert_eq!(code, exit_codes::CLEAN);
        assert_eq!(synced_code, std::process::ExitCode::SUCCESS);
        assert_eq!(lines, vec!["draft-aaaaaa\tpromoted\tsynced\tREC-1"]);
        let file = "meta/work/REC-1-title.md";
        assert!(promoted.read(file).is_some());
        assert_eq!(promoted.read(file), synced.read(file));
    }

    #[test]
    fn adopt_an_existing_issue_without_creating_and_clears_the_marker() {
        let repo = Repo::new();
        repo.save_attempted();
        let tracker = held("PP-900");

        let (lines, _, code) =
            reported(repo.promote(&tracker, Some("PP-900"), false));

        assert_eq!(code, exit_codes::CLEAN, "{lines:?}");
        assert_eq!(creates(&tracker), 0);
        assert!(!repo.record_path().exists());
        assert!(repo.read("meta/work/PP-900-title.md").is_some());
        assert_eq!(repo.read(DRAFT_FILE), None);
    }

    #[test]
    fn adopt_a_missing_issue_exits_non_zero_and_changes_nothing() {
        let repo = Repo::new();
        repo.save_attempted();
        let draft_before = repo.read(DRAFT_FILE);
        let record_before =
            std::fs::read_to_string(repo.record_path()).unwrap();
        let tracker = Rc::new(
            RecordingTracker::holding(Vec::new())
                .not_found(&ExternalId::new("PP-900".to_owned())),
        );

        let (lines, _, code) =
            reported(repo.promote(&tracker, Some("PP-900"), false));

        assert_eq!(code, exit_codes::UNRESOLVED);
        assert_eq!(
            lines[0],
            "draft-aaaaaa\tnot-promoted\tunsynced\tadopted-issue-missing"
        );
        assert_eq!(repo.read(DRAFT_FILE), draft_before);
        assert_eq!(
            std::fs::read_to_string(repo.record_path()).unwrap(),
            record_before
        );
    }

    #[test]
    fn adopt_a_key_held_by_a_local_item_names_both_and_suggests_removal() {
        let repo = Repo::new();
        repo.write(
            "meta/work/0042-legacy.md",
            "---\nid: \"0042\"\nexternal_id: \"PP-900\"\n---\n\n# 0042: Legacy\n",
        );

        let (lines, remedy, code) =
            reported(repo.promote(&held("PP-900"), Some("PP-900"), false));

        assert_eq!(code, exit_codes::UNRESOLVED);
        assert_eq!(
            lines[0],
            "draft-aaaaaa\tnot-promoted\tunsynced\tkey-linked"
        );
        let remedy = remedy.expect("a remedy on stderr");
        assert!(remedy.contains("0042-legacy.md"), "{remedy}");
        assert!(remedy.contains(DRAFT), "{remedy}");
        assert!(remedy.contains("delete the draft"), "{remedy}");
    }

    #[test]
    fn create_accepts_the_duplicate_risk_and_clears_the_marker() {
        let repo = Repo::new();
        repo.save_attempted();
        let tracker = fresh();

        let (lines, _, code) = reported(repo.promote(&tracker, None, true));

        assert_eq!(code, exit_codes::CLEAN, "{lines:?}");
        assert_eq!(creates(&tracker), 1);
        assert!(!repo.record_path().exists());
        assert!(repo.read("meta/work/REC-1-title.md").is_some());
    }

    #[test]
    fn promoting_an_already_promoted_draft_reports_its_key_and_exits_zero() {
        let repo = Repo::new();
        let tracker = fresh();
        reported(repo.promote(&tracker, None, false));

        let (lines, _, code) = reported(repo.promote(&tracker, None, false));

        assert_eq!(code, exit_codes::CLEAN);
        assert_eq!(
            lines,
            vec!["draft-aaaaaa\talready-promoted\tsynced\tREC-1"]
        );
        assert_eq!(creates(&tracker), 1);
    }

    #[test]
    fn promoting_a_non_draft_is_e_promote_not_a_draft() {
        let repo = Repo::new();
        repo.write(
            "meta/work/0042-legacy.md",
            "---\nid: \"0042\"\n---\n\n# 0042: Legacy\n",
        );

        for input in ["0042", "draft-zzzzzz"] {
            let outcome = repo.promote_named(input, &fresh(), None, false);

            let PromoteOutcome::NotADraft(message) = outcome else {
                panic!("{input} is not a draft: {outcome:?}");
            };
            assert!(message.starts_with("E_PROMOTE_NOT_A_DRAFT"), "{message}");
        }
    }

    #[test]
    fn promoting_under_a_local_pattern_is_refused_before_any_create() {
        let repo = Repo::new();
        repo.write(
            ".accelerator/config.md",
            "---\nwork:\n  integration: jira\n---\n",
        );
        let tracker = fresh();

        let outcome = repo.promote_named(DRAFT, &tracker, None, false);

        let PromoteOutcome::Failed { message, code } = outcome else {
            panic!("the promotion was not refused: {outcome:?}");
        };
        assert!(
            message.starts_with("E_PROMOTE_NOT_TRACKER_OWNED"),
            "{message}"
        );
        assert_eq!(code, exit_codes::USAGE);
        assert_eq!(creates(&tracker), 0);
    }

    #[test]
    fn a_draft_id_two_items_claim_is_refused_naming_both() {
        let repo = Repo::new();
        repo.write(
            "meta/work/PP-900-title.md",
            "---\nid: \"PP-900\"\naliases: [\"draft-aaaaaa\"]\n---\n\n\
             # PP-900: Title\n",
        );
        let tracker = fresh();

        let outcome = repo.promote_named(DRAFT, &tracker, None, false);

        let PromoteOutcome::Failed { message, code } = outcome else {
            panic!("the promotion was not refused: {outcome:?}");
        };
        assert!(message.starts_with("E_PROMOTE_CONFLICTING"), "{message}");
        assert!(message.contains("PP-900-title.md"), "{message}");
        assert!(message.contains(DRAFT_FILE), "{message}");
        assert_eq!(code, exit_codes::USAGE);
        assert_eq!(creates(&tracker), 0);
    }

    #[test]
    fn adopt_replaces_an_unreadable_record() {
        let repo = Repo::new();
        std::fs::create_dir_all(repo.record_path().parent().unwrap()).unwrap();
        std::fs::write(repo.record_path(), "{\"kind\":").unwrap();

        let (lines, _, code) =
            reported(repo.promote(&held("PP-900"), Some("PP-900"), false));

        assert_eq!(code, exit_codes::CLEAN, "{lines:?}");
        assert!(repo.read("meta/work/PP-900-title.md").is_some());
        assert!(!repo.record_path().exists());
    }

    #[test]
    fn work_promote_first_finishes_an_interrupted_key_change_retirement() {
        let repo = Repo::new();
        repo.write(
            "meta/work/PP-760-title.md",
            "---\nid: \"PP-760\"\ntitle: \"Title\"\nexternal_id: \
             \"PP-760\"\n---\n\n# PP-760: Title\n",
        );
        let store = FileCorpusStore::new(repo.root());
        FileRetirementRecords::new(&repo.path(".accelerator/state"), &store)
            .save(&RetirementRecord::of(&Retirement {
                old_id: "PP-760",
                new_id: "ENG-42",
                new_external_id: Some("ENG-42"),
            }))
            .unwrap();

        let (lines, _, code) = reported(repo.promote(&fresh(), None, false));

        assert_eq!(code, exit_codes::CLEAN, "{lines:?}");
        assert_eq!(lines[0], "PP-760\tresumed\t-\tPP-760->ENG-42");
        assert!(repo.read("meta/work/ENG-42-title.md").is_some());
        assert_eq!(repo.read("meta/work/PP-760-title.md"), None);
        assert!(repo.read("meta/work/REC-1-title.md").is_some());
    }
}
