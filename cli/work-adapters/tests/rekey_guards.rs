//! What an earlier sync or re-key left behind that must be settled before a
//! re-key may start.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::RecoveryCopies as _;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::FileRecoveryCopies;
use tracker::ExternalId;
use work::draft_id::DraftId;
use work::identity::ItemIdentity;
use work::promotion::IntendedBaseline;
use work::promotion::LocalDigests;
use work::promotion::PromotionRecord;
use work::promotion::PromotionStage;
use work::promotion::RemoteHash;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::rekey::outcome::SyncBlocker;
use work::retirement::Retirement;
use work::retirement::RetirementRecord;
use work::sync::PendingPush;
use work::sync::RequestFingerprint;
use work::tracker_key::TrackerKey;
use work_adapters::rekey::guards::pending_pushes;
use work_adapters::rekey::guards::unfinished_sync_work;
use work_adapters::rekey::guards::unsettled_recoveries;
use work_adapters::rekey::guards::PendingPushDir;
use work_adapters::rekey::guards::SyncState;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::retirement_records::RetirementRecords as _;
use work_adapters::sync::pending_push;

type TestError = Box<dyn std::error::Error>;

const INTEGRATION: &str = "linear";

struct Repo {
    dir: tempfile::TempDir,
    writer: FileCorpusStore,
    recovery: FileRecoveryCopies,
}

impl Repo {
    fn new() -> Result<Self, TestError> {
        let dir = tempfile::tempdir()?;
        let writer = FileCorpusStore::new(dir.path());
        let recovery = FileRecoveryCopies::new(
            dir.path().join(".accelerator/state"),
            dir.path(),
        );
        Ok(Self {
            dir,
            writer,
            recovery,
        })
    }

    fn state(&self) -> PathBuf {
        self.dir.path().join(".accelerator/state")
    }

    fn integrations(&self) -> PathBuf {
        self.dir.path().join(".accelerator/state/integrations")
    }

    fn records(&self) -> FileRetirementRecords<'_> {
        FileRetirementRecords::new(&self.state(), &self.writer)
    }

    fn marker(&self, name: &str) -> PathBuf {
        pending_push::path(&self.integrations(), INTEGRATION, name)
    }

    fn write_marker(&self, name: &str, content: &str) -> Result<(), TestError> {
        pending_push::prepare_dir(&self.integrations(), INTEGRATION)?;
        fs::write(self.marker(name), content)?;
        Ok(())
    }
}

fn with_sync_state<T>(
    repo: &Repo,
    integrated: bool,
    check: impl FnOnce(&SyncState<'_>) -> T,
) -> T {
    let state = repo.state();
    let integrations = repo.integrations();
    let records = repo.records();
    check(&SyncState {
        state_dir: &state,
        records: &records,
        recovery: &repo.recovery,
        pending_push: integrated.then_some(PendingPushDir {
            integrations_root: &integrations,
            integration: INTEGRATION,
        }),
    })
}

fn request() -> RequestFingerprint {
    RequestFingerprint {
        title: "Add search".to_owned(),
        digest: "digest".to_owned(),
        attempted_at: 1,
        failure: None,
    }
}

fn tracker_key() -> Result<TrackerKey, TestError> {
    Ok(TrackerKey::parse("ENG-42").ok_or("a tracker key")?)
}

fn draft() -> Result<DraftId, TestError> {
    Ok(DraftId::parse("draft-k3x9mq").ok_or("a draft ID")?)
}

fn record(stage: PromotionStage) -> Result<PromotionRecord, TestError> {
    Ok(PromotionRecord {
        draft_id: draft()?,
        request: request(),
        content_digest: "digest".to_owned(),
        stage,
    })
}

fn created() -> Result<PromotionStage, TestError> {
    Ok(PromotionStage::Created {
        key: tracker_key()?,
        created_remote_hash: None,
    })
}

fn retiring() -> Result<PromotionStage, TestError> {
    Ok(PromotionStage::Retiring {
        key: tracker_key()?,
        baseline: Box::new(IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local: LocalDigests {
                legacy: "draft".to_owned(),
                fingerprints: None,
            },
        }),
        recovery_dir: PathBuf::from("retirement-recovery/draft-k3x9mq--ENG-42"),
        before: Box::new(created()?),
    })
}

fn legacy_marker() -> String {
    pending_push::render(&PendingPush::Created {
        request: request(),
        external_id: ExternalId::new("ENG-42".to_owned()),
    })
}

fn item(path: &str, id: &str) -> ItemIdentity {
    ItemIdentity {
        path: PathBuf::from(path),
        id: id.to_owned(),
        aliases: Vec::new(),
        external_id: None,
    }
}

fn items() -> Vec<ItemIdentity> {
    vec![
        item("meta/work/0042-add-search.md", "0042"),
        item("meta/work/drafts/draft-k3x9mq-idea.md", "draft-k3x9mq"),
        item("meta/work/ACC-0007-untouched.md", "ACC-0007"),
    ]
}

fn renumbered(old: &str, new: &str) -> RekeyEntry {
    RekeyEntry {
        old: old.to_owned(),
        target: RekeyTarget::Renumbered(new.to_owned()),
    }
}

fn entries() -> Vec<RekeyEntry> {
    vec![
        renumbered("0042", "ACC-0042"),
        renumbered("draft-k3x9mq", "ACC-0043"),
    ]
}

#[test]
fn an_outstanding_retirement_record_blocks_naming_its_old_id(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.records().save(&RetirementRecord::of(&Retirement {
        old_id: "PP-760",
        new_id: "ENG-42",
        new_external_id: Some("ENG-42"),
    }))?;

    let blockers = with_sync_state(&repo, true, unfinished_sync_work);

    assert_eq!(
        blockers,
        vec![SyncBlocker::Retirement {
            old: "PP-760".to_owned(),
            new: "ENG-42".to_owned(),
        }]
    );
    Ok(())
}

#[test]
fn a_malformed_retirement_record_blocks() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let dir = repo.state().join("retirement-records");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("PP-1--ENG-1.json"), "{")?;

    let blockers = with_sync_state(&repo, false, unfinished_sync_work);

    assert!(
        matches!(blockers.as_slice(), [SyncBlocker::UnreadableRecord(_)]),
        "{blockers:?}"
    );
    Ok(())
}

#[test]
fn a_retiring_promotion_record_blocks_naming_its_draft() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    repo.write_marker(
        "draft-k3x9mq",
        &pending_push::render_record(&record(retiring()?)?),
    )?;

    let blockers = with_sync_state(&repo, true, unfinished_sync_work);

    assert_eq!(
        blockers,
        vec![SyncBlocker::Promotion {
            draft: "draft-k3x9mq".to_owned(),
        }]
    );
    Ok(())
}

#[test]
fn a_pending_push_marker_on_a_rekeyed_item_blocks_naming_it(
) -> Result<(), TestError> {
    for (marker, content, blocked) in [
        ("0042", legacy_marker(), "meta/work/0042-add-search.md"),
        (
            "add-search",
            legacy_marker(),
            "meta/work/0042-add-search.md",
        ),
        (
            "draft-k3x9mq",
            pending_push::render_record(&record(created()?)?),
            "meta/work/drafts/draft-k3x9mq-idea.md",
        ),
    ] {
        let repo = Repo::new()?;
        repo.write_marker(marker, &content)?;

        let pushes = with_sync_state(&repo, true, |sync| {
            pending_pushes(sync, &entries(), &items())
        });

        assert_eq!(pushes, vec![PathBuf::from(blocked)], "{marker}");
    }
    Ok(())
}

#[test]
fn a_pending_push_marker_on_an_untouched_item_does_not_block(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write_marker("ACC-0007", &legacy_marker())?;
    repo.write_marker("untouched", &legacy_marker())?;

    let pushes = with_sync_state(&repo, true, |sync| {
        pending_pushes(sync, &entries(), &items())
    });

    assert!(pushes.is_empty(), "{pushes:?}");
    Ok(())
}

#[test]
fn an_unreadable_marker_blocks_naming_its_file() -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write_marker("ACC-0007", "{")?;

    let pushes = with_sync_state(&repo, true, |sync| {
        pending_pushes(sync, &entries(), &items())
    });

    assert_eq!(pushes, vec![repo.marker("ACC-0007")]);
    Ok(())
}

fn recovery_child(repo: &Repo, name: &str) -> Result<PathBuf, TestError> {
    let dir = Path::new("retirement-recovery").join(name);
    repo.recovery.prepare(&dir)?;
    Ok(dir)
}

#[test]
fn a_marker_less_recovery_child_is_partly_applied() -> Result<(), TestError> {
    let repo = Repo::new()?;
    recovery_child(&repo, "0042--ACC-0042")?;

    let unsettled = with_sync_state(&repo, true, unsettled_recoveries);

    assert_eq!(
        unsettled,
        vec![repo.state().join("retirement-recovery/0042--ACC-0042")]
    );
    Ok(())
}

#[test]
fn a_restore_pending_recovery_child_is_partly_applied() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    let dir = recovery_child(&repo, "0042--ACC-0042")?;
    repo.recovery
        .mark_restore_pending(&dir, &[PathBuf::from("meta/work/a.md")])?;

    let unsettled = with_sync_state(&repo, true, unsettled_recoveries);

    assert_eq!(
        unsettled,
        vec![repo.state().join("retirement-recovery/0042--ACC-0042")]
    );
    Ok(())
}

#[test]
fn a_completed_recovery_child_is_ignored() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let dir = recovery_child(&repo, "0042--ACC-0042")?;
    repo.recovery.mark_restore_pending(&dir, &[])?;
    repo.recovery.mark_completed(&dir)?;

    let unsettled = with_sync_state(&repo, true, unsettled_recoveries);

    assert!(unsettled.is_empty(), "{unsettled:?}");
    Ok(())
}

#[test]
fn a_bare_recovery_parent_is_ignored() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let parent = repo.state().join("retirement-recovery");
    fs::create_dir_all(&parent)?;
    fs::write(parent.join(".gitignore"), "*\n")?;

    let unsettled = with_sync_state(&repo, true, unsettled_recoveries);

    assert!(unsettled.is_empty(), "{unsettled:?}");
    Ok(())
}

#[test]
fn recovery_children_are_judged_the_same_without_an_integration(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    recovery_child(&repo, "0041--ACC-0041")?;
    let completed = recovery_child(&repo, "0042--ACC-0042")?;
    repo.recovery.mark_restore_pending(&completed, &[])?;
    repo.recovery.mark_completed(&completed)?;

    let integrated = with_sync_state(&repo, true, unsettled_recoveries);
    let local = with_sync_state(&repo, false, unsettled_recoveries);

    assert_eq!(integrated, local);
    assert_eq!(
        local,
        vec![repo.state().join("retirement-recovery/0041--ACC-0041")]
    );
    Ok(())
}
