//! The re-key run plan as one JSON file in an ignored state directory.

// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#![allow(clippy::literal_string_with_formatting_args)]

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use corpus::WorkItemIdScheme;
use corpus_adapters::FileCorpusStore;
use work::draft_id::DraftId;
use work::identity::ItemIdentity;
use work::identity_run::RunPlanGate;
use work::identity_run::RunPlanStatus as _;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::ProposedPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::run_plan::Unreadability;
use work::rekey::run_plan::UnreadablePlan;
use work::rekey::run_plan::RUN_PLAN_SCHEMA;
use work::rekey::write_set::Base;
use work::rekey::write_set::Digest;
use work::rekey::write_set::Recorded;
use work::rekey::write_set::StartState;
use work::rekey::write_set::WriteSet;
use work::tracker_key::TrackerKey;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;
use work_adapters::rekey::run_plan_store::RekeyStatus;
use work_adapters::rekey::run_plan_store::RunPlanStore as _;

type TestError = Box<dyn std::error::Error>;

const PLAN: &str = ".accelerator/state/rekey/run-plan.json";

struct Repo {
    dir: tempfile::TempDir,
    writer: RecordingWrite,
}

impl Repo {
    fn new() -> Result<Self, TestError> {
        let dir = tempfile::tempdir()?;
        let writer = RecordingWrite::new(dir.path());
        Ok(Self { dir, writer })
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn state(&self) -> PathBuf {
        self.root().join(".accelerator/state")
    }

    fn store(&self) -> FileRunPlanStore<'_> {
        FileRunPlanStore::new(
            &self.state(),
            self.root(),
            &self.writer,
            &self.writer.inner,
        )
    }

    fn plan_path(&self) -> PathBuf {
        self.root().join(PLAN)
    }
}

struct RecordingWrite {
    inner: FileCorpusStore,
    written: RefCell<Vec<PathBuf>>,
}

impl RecordingWrite {
    fn new(root: &Path) -> Self {
        Self {
            inner: FileCorpusStore::new(root),
            written: RefCell::new(Vec::new()),
        }
    }
}

impl AtomicWrite for RecordingWrite {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.written.borrow_mut().push(path.to_path_buf());
        self.inner.write(path, bytes)
    }
}

fn scheme() -> WorkItemIdScheme {
    WorkItemIdScheme {
        id_pattern: "{key}-{number:04d}".to_owned(),
        key: Some("ACC".to_owned()),
    }
}

fn identity(
    root: &Path,
    path: &str,
    id: &str,
    aliases: &[&str],
    external_id: Option<&str>,
) -> ItemIdentity {
    ItemIdentity {
        path: root.join(path),
        id: id.to_owned(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
        external_id: external_id.map(str::to_owned),
    }
}

fn fixture_items(root: &Path) -> Vec<ItemIdentity> {
    vec![
        identity(root, "meta/work/0042-search.md", "0042", &[], None),
        identity(
            root,
            "meta/work/PP-760-sync.md",
            "PP-760",
            &["0007"],
            Some("PP-760"),
        ),
        identity(
            root,
            "meta/work/drafts/draft-k3x9mq-idea.md",
            "draft-k3x9mq",
            &[],
            None,
        ),
    ]
}

fn renumbered(old: &str, new: &str) -> RekeyEntry {
    RekeyEntry {
        old: old.to_owned(),
        target: RekeyTarget::Renumbered(new.to_owned()),
    }
}

fn fixture_plan(root: &Path) -> InProgressPlan {
    InProgressPlan::new(
        IdentityFingerprint::of(&scheme(), &fixture_items(root)),
        vec![
            renumbered("0042", "ACC-0042"),
            renumbered("PP-760", "ACC-0043"),
            renumbered("draft-k3x9mq", "ACC-0044"),
        ],
        WriteSet::default().extended_with([
            (
                "meta/work/0042-search.md".to_owned(),
                Recorded {
                    state: StartState::Content(Digest::new(
                        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
                    )),
                    base: Some(Base::new(vec!["c0ffee".to_owned()])),
                },
            ),
            (
                "meta/work/ACC-0042-search.md".to_owned(),
                Recorded {
                    state: StartState::Absent,
                    base: Some(Base::new(vec!["c0ffee".to_owned()])),
                },
            ),
            (
                "meta/notes/forced.md".to_owned(),
                Recorded {
                    state: StartState::Content(Digest::new(
                        "60303ae22b998861bce3b28f33eec1be758a213c86c93c076dbe9f558c11c752",
                    )),
                    base: None,
                },
            ),
        ]),
    )
}

fn every_target_kind(root: &Path) -> Result<ProposedPlan, TestError> {
    let key = TrackerKey::parse("ENG-42").ok_or("a tracker key")?;
    let draft = DraftId::parse("draft-a1b2cd").ok_or("a draft ID")?;
    Ok(ProposedPlan::new(
        IdentityFingerprint::of(&scheme(), &fixture_items(root)),
        vec![
            renumbered("0042", "ACC-0042"),
            RekeyEntry {
                old: "PP-760".to_owned(),
                target: RekeyTarget::TrackerKey(key),
            },
            RekeyEntry {
                old: "0007".to_owned(),
                target: RekeyTarget::Draft(draft),
            },
        ],
    ))
}

fn write_plan(repo: &Repo, content: &str) -> Result<(), TestError> {
    fs::create_dir_all(repo.state().join("rekey"))?;
    fs::write(repo.plan_path(), content)?;
    Ok(())
}

#[test]
fn a_missing_directory_loads_as_no_plan_and_creates_nothing(
) -> Result<(), TestError> {
    let repo = Repo::new()?;

    assert_eq!(repo.store().load(), Ok(None));
    assert_eq!(repo.store().status(), RekeyStatus::Absent);
    assert!(!repo.state().exists());
    Ok(())
}

#[test]
fn saving_writes_a_verified_ignore_everything_rule() -> Result<(), TestError> {
    let repo = Repo::new()?;

    repo.store()
        .save(&RunPlan::InProgress(fixture_plan(repo.root())))?;

    assert_eq!(
        fs::read_to_string(repo.state().join("rekey/.gitignore"))?,
        "*\n"
    );
    Ok(())
}

#[test]
fn saving_replaces_the_file_atomically() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let proposed = RunPlan::Proposed(every_target_kind(repo.root())?);
    let started = RunPlan::InProgress(fixture_plan(repo.root()));

    repo.store().save(&proposed)?;
    repo.store().save(&started)?;

    assert_eq!(
        repo.writer
            .written
            .borrow()
            .iter()
            .filter(|path| **path == repo.plan_path())
            .count(),
        2
    );
    assert_eq!(repo.store().load(), Ok(Some(started)));
    Ok(())
}

#[test]
fn a_malformed_plan_is_unreadable_naming_the_file() -> Result<(), TestError> {
    let repo = Repo::new()?;
    write_plan(&repo, "{\"schema\": 1, \"state\": ")?;

    let loaded = repo.store().load();

    assert!(matches!(
        loaded,
        Err(UnreadablePlan { ref path, cause: Unreadability::Malformed(_) })
            if *path == repo.plan_path()
    ));
    Ok(())
}

#[test]
fn a_plan_of_a_newer_schema_is_unreadable_naming_the_version(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let newer = RUN_PLAN_SCHEMA + 1;
    write_plan(&repo, &format!("{{\"schema\": {newer}, \"novel\": true}}"))?;

    assert_eq!(
        repo.store().load(),
        Err(UnreadablePlan {
            path: repo.plan_path(),
            cause: Unreadability::NewerSchema(newer),
        })
    );
    Ok(())
}

#[test]
fn a_saved_plan_round_trips() -> Result<(), TestError> {
    let repo = Repo::new()?;
    for plan in [
        RunPlan::Proposed(every_target_kind(repo.root())?),
        RunPlan::InProgress(fixture_plan(repo.root())),
    ] {
        repo.store().save(&plan)?;
        assert_eq!(repo.store().load(), Ok(Some(plan)));
    }
    Ok(())
}

#[test]
fn an_older_schema_plan_is_read() -> Result<(), TestError> {
    let repo = Repo::new()?;
    write_plan(&repo, include_str!("fixtures/rekey/run-plan-schema-1.json"))?;

    assert_eq!(
        repo.store().load(),
        Ok(Some(RunPlan::InProgress(fixture_plan(repo.root()))))
    );
    Ok(())
}

#[test]
fn status_reports_absent_proposed_in_progress_and_unreadable(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    assert_eq!(repo.store().status(), RekeyStatus::Absent);
    assert_eq!(repo.store().gate(), RunPlanGate::Clear);

    repo.store()
        .save(&RunPlan::Proposed(every_target_kind(repo.root())?))?;
    assert_eq!(repo.store().status(), RekeyStatus::Proposed);
    assert_eq!(repo.store().gate(), RunPlanGate::Clear);

    repo.store()
        .save(&RunPlan::InProgress(fixture_plan(repo.root())))?;
    assert_eq!(repo.store().status(), RekeyStatus::InProgress);
    assert_eq!(repo.store().gate(), RunPlanGate::InProgress);

    write_plan(&repo, "not json")?;
    assert!(matches!(repo.store().status(), RekeyStatus::Unreadable(_)));
    assert!(matches!(repo.store().gate(), RunPlanGate::Unreadable(_)));
    Ok(())
}

#[test]
fn removing_the_plan_leaves_none_and_removing_none_succeeds(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.store()
        .save(&RunPlan::InProgress(fixture_plan(repo.root())))?;

    repo.store().remove()?;
    assert_eq!(repo.store().load(), Ok(None));
    repo.store().remove()?;
    Ok(())
}

#[test]
fn setting_a_plan_aside_keeps_it_beside_the_plan_under_a_timestamp(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    write_plan(&repo, "not json")?;

    let aside = repo.store().set_aside()?;

    assert_eq!(repo.store().load(), Ok(None));
    assert_eq!(aside.parent(), repo.plan_path().parent());
    let name = aside
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("a file name")?;
    assert!(name.starts_with("run-plan.discarded-"), "{name}");
    assert_eq!(
        aside.extension().and_then(|extension| extension.to_str()),
        Some("json")
    );
    assert_eq!(fs::read_to_string(&aside)?, "not json");
    Ok(())
}
