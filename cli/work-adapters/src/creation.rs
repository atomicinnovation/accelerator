//! Creating one work item, never sending a second create for it.
//!
//! The item is locally numbered and pushed through its slug-named marker
//! when asked, kept as a draft, or under `{tracker}` written as a draft and
//! promoted onto the issue the tracker creates. Whatever is interrupted, a
//! rerun finds the earlier create rather than repeating it.

use std::path::Path;
use std::path::PathBuf;

use corpus::lock::ExclusiveLock;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::scan::FileReader;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::AtomicWrite;
use tracker::ExternalId;
use tracker::RemoteTracker;
use work::draft_id::mint_draft_id;
use work::draft_id::DraftId;
use work::draft_id::SuffixDraws;
use work::identity::linker_of;
use work::identity::ItemIdentity;
use work::promotion::IntendedBaseline;
use work::promotion::NotPromoted;
use work::promotion::Promotion;
use work::promotion::PromotionStage;
use work::promotion::ReadBack;
use work::promotion::RemoteHash;
use work::promotion::ID_PLACEHOLDER;
use work::retirement::Retirement;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRefusal;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushOutcome;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;
use work::tracker_key::TrackerKey;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;

use crate::create_request_fields;
use crate::filesystem::drafts_dir;
use crate::filesystem::FilesystemWorkItemFiles;
use crate::promotion::Detail;
use crate::promotion::PromotionOutcome;
use crate::promotion::PromotionRow;
use crate::promotion_records::FilePromotionRecords;
use crate::promotion_records::PromotionRecords;
use crate::remote_create::send_create;
use crate::remote_create::CreateRequest;
use crate::remote_create::RemoteCreate;
use crate::sync::baseline;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::created_baseline::record_created_baseline;
use crate::sync::digest;
use crate::sync::pending_push;
use crate::sync::pending_push::Marker;

/// The file store every creation strategy writes through.
pub trait CreationStore: AtomicWrite + ExclusiveCreate + FileRemove {}

impl<T: AtomicWrite + ExclusiveCreate + FileRemove> CreationStore for T {}

/// The item being created, rendered for whichever ID it comes to take.
pub trait NewItem {
    fn title(&self) -> &str;
    fn kind(&self) -> &str;
    fn slug(&self) -> String;

    /// # Errors
    /// A message when the body cannot be rendered.
    fn body(&self, id: &str) -> Result<String, String>;

    /// # Errors
    /// A message when the item cannot be rendered.
    fn content(
        &self,
        id: &str,
        external_id: Option<&str>,
    ) -> Result<String, String>;
}

/// Where an integration keeps its markers, promotion records and baseline.
pub struct IntegrationState {
    pub root: PathBuf,
    pub name: String,
}

impl IntegrationState {
    fn baseline_path(&self) -> PathBuf {
        baseline::path(&self.root, &self.name)
    }

    /// The integration's promotion records, written through `store_at`.
    #[must_use]
    pub fn promotion_records<'a>(
        &self,
        store: &'a dyn CreationStore,
    ) -> FilePromotionRecords<'a> {
        FilePromotionRecords::new(&self.root, &self.name, store)
    }
}

/// A file store bounded to the directory it is opened at.
pub type StoreAt<'a> = dyn Fn(&Path) -> Box<dyn CreationStore> + 'a;

/// Told of each draft as soon as it is on disk; an error stops the draft
/// being pushed, so nothing reaches the tracker that the caller could not
/// record.
pub type Drafted<'a> = dyn FnMut(&DraftId) -> Result<(), String> + 'a;

/// The tracker a legacy push sends to, or the outcome when none can be had.
pub type LegacyTracker<'a> =
    dyn Fn() -> Result<Box<dyn RemoteTracker>, PushOutcome> + 'a;

/// Promotes the just-written draft, or `None` when no tracker can be had.
pub type DraftPromotion<'a> =
    dyn FnMut(&DraftId, PathBuf) -> Result<Option<PromotionRow>, String> + 'a;

/// The corpus one item is created in.
pub struct Creation<'a> {
    pub item: &'a dyn NewItem,
    pub repo_root: &'a Path,
    pub work_dir: &'a Path,
    pub state_dir: &'a Path,
    pub store_at: &'a StoreAt<'a>,
    pub reader: &'a dyn FileReader,
    pub lock: &'a dyn ExclusiveLock,
    pub now: u64,
}

pub struct PushReport {
    pub outcome: PushOutcome,
    pub external_id: Option<String>,
    /// What stderr tells the user beside the keyword: a rejected request's
    /// cause, a blocking item and its remedy, or the paths to restore.
    pub cause: Option<String>,
    /// The blocking item, or each path to restore, for a caller that
    /// reports them as data.
    pub details: Vec<Detail>,
}

/// What a creation strategy left on disk, and what became of its push.
pub struct CreationOutcome {
    pub path: Option<PathBuf>,
    pub push: Option<PushReport>,
}

impl CreationOutcome {
    const fn unpushed(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            push: None,
        }
    }
}

/// Why a create wrote nothing: an earlier create of the same content is
/// still pending, or something failed.
pub enum CreateFailure {
    Pending {
        message: String,
        existing_draft: Option<PathBuf>,
    },
    Failed(String),
}

impl From<String> for CreateFailure {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

fn pushed(
    path: Option<PathBuf>,
    outcome: PushOutcome,
    key: Option<&ExternalId>,
    cause: Option<String>,
) -> CreationOutcome {
    CreationOutcome {
        path,
        push: Some(PushReport {
            outcome,
            external_id: key.map(|key| key.as_str().to_owned()),
            cause,
            details: Vec::new(),
        }),
    }
}

impl Creation<'_> {
    fn hold_create_lock(&self) -> Result<HeldLock, String> {
        self.lock.acquire(&LockName::Create).map_err(|error| {
            format!("could not acquire the work-item creation lock: {error}")
        })
    }

    fn item_store(&self) -> Box<dyn CreationStore> {
        (self.store_at)(self.repo_root)
    }

    fn exists(&self, path: &Path) -> bool {
        matches!(self.reader.read(path), Ok(Some(_)))
    }

    fn refuse_to_overwrite(&self, target: &Path) -> Result<(), String> {
        if self.exists(target) {
            return Err(format!(
                "refusing to overwrite an existing file: {}",
                target.display()
            ));
        }
        Ok(())
    }

    fn write_new(
        &self,
        store: &dyn CreationStore,
        target: &Path,
        content: &str,
    ) -> Result<(), String> {
        let dir = target.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir).map_err(|error| {
            format!("could not create {}: {error}", dir.display())
        })?;
        self.refuse_to_overwrite(target)?;
        AtomicWrite::write(store, target, content.as_bytes())
            .map_err(|error| error.to_string())
    }

    /// Once an issue exists, a failed write leaves it carried by no item,
    /// so one transient failure is worth absorbing.
    fn write_new_retrying_once(
        &self,
        store: &dyn CreationStore,
        target: &Path,
        content: &str,
    ) -> Result<(), String> {
        self.write_new(store, target, content)
            .or_else(|_| self.write_new(store, target, content))
    }

    fn identities(&self) -> Result<Vec<ItemIdentity>, String> {
        corpus_identities(self.work_dir)
    }

    fn target_for(&self, id: &str) -> PathBuf {
        self.work_dir.join(format!("{id}-{}.md", self.item.slug()))
    }

    /// Writes the draft, minting an ID no item holds. The create lock must
    /// be held.
    fn write_draft(
        &self,
        draws: &mut dyn SuffixDraws,
        _held: &HeldLock,
    ) -> Result<(DraftId, PathBuf), String> {
        let draft = mint_draft_id(draws, &self.identities()?)
            .map_err(|error| error.to_string())?;
        let target = drafts_dir(self.work_dir).join(format!(
            "{}-{}.md",
            draft.as_str(),
            self.item.slug()
        ));
        self.write_new(
            self.item_store().as_ref(),
            &target,
            &self.item.content(draft.as_str(), None)?,
        )?;
        Ok((draft, target))
    }
}

fn corpus_identities(work_dir: &Path) -> Result<Vec<ItemIdentity>, String> {
    FilesystemWorkItemFiles::new(work_dir)
        .files()
        .map(|files| identities(&files))
        .map_err(|error| error.to_string())
}

fn carries_external_id(
    corpus: &[ItemIdentity],
    external_id: &ExternalId,
) -> bool {
    linker_of(external_id.as_str(), corpus).is_some()
}

/// Writes the item as a draft no tracker has confirmed.
///
/// # Errors
///
/// A message when the draft cannot be written or `drafted` refuses it.
pub fn save_draft(
    creation: &Creation<'_>,
    draws: &mut dyn SuffixDraws,
    drafted: &mut Drafted<'_>,
) -> Result<CreationOutcome, String> {
    let held = creation.hold_create_lock()?;
    let (draft, path) = creation.write_draft(draws, &held)?;
    drop(held);
    drafted(&draft)?;
    Ok(CreationOutcome::unpushed(path))
}

/// A legacy push's result: the key it obtained, if any, the tracker it
/// reached, and the marker to spend once the item carries the key.
struct LegacyPush {
    outcome: PushOutcome,
    key: Option<ExternalId>,
    cause: Option<String>,
    tracker: Option<Box<dyn RemoteTracker>>,
    marker: Option<PathBuf>,
}

impl LegacyPush {
    const fn without_issue(
        outcome: PushOutcome,
        cause: Option<String>,
    ) -> Self {
        Self {
            outcome,
            key: None,
            cause,
            tracker: None,
            marker: None,
        }
    }
}

/// How a locally numbered item is pushed: the integration it goes to and
/// the tracker the push sends through.
pub struct LegacyPushing<'a> {
    pub state: &'a IntegrationState,
    pub tracker: &'a LegacyTracker<'a>,
}

fn refusal_message(
    reason: RefusalReason,
    marker_path: &Path,
    marker: Option<&PendingPush>,
) -> String {
    match reason {
        RefusalReason::MarkerUnreadable => format!(
            "E_PUSH_MARKER_UNREADABLE: the pending-push marker at {} could \
             not be parsed; a previous create may have partially applied. \
             Inspect or remove the marker before retrying.",
            marker_path.display()
        ),
        RefusalReason::PriorAttemptUnknownOutcome => {
            let Some(PendingPush::Attempted { request }) = marker else {
                unreachable!("PriorAttemptUnknownOutcome always carries an Attempted marker")
            };
            format!(
                "E_PUSH_PENDING: a previous create attempt for '{}' at {} \
                 (recorded at {}{}) has an unknown outcome — a remote \
                 issue may already exist. Inspect it, then remove the \
                 marker to retry.",
                request.title,
                marker_path.display(),
                request.attempted_at,
                request
                    .failure
                    .as_deref()
                    .map(|detail| format!(", failure: {detail}"))
                    .unwrap_or_default()
            )
        }
        RefusalReason::FingerprintMismatch => format!(
            "E_PUSH_FINGERPRINT_MISMATCH: the pending-push marker at {} was \
             recorded for a different request with the same title but a \
             different body or kind. Remove the marker to force a new \
             create.",
            marker_path.display()
        ),
        RefusalReason::AlreadyWritten => {
            let Some(PendingPush::Created { external_id, .. }) = marker else {
                unreachable!("AlreadyWritten always carries a Created marker")
            };
            format!(
                "E_PUSH_ALREADY_WRITTEN: the pending-push marker at {} \
                 claims external_id '{}', already carried by a work item \
                 on disk. Remove the marker if this is a genuine duplicate \
                 create.",
                marker_path.display(),
                external_id.as_str()
            )
        }
    }
}

/// The marker a legacy push records its request and outcome in.
struct LegacyMarker {
    path: PathBuf,
    store: Box<dyn CreationStore>,
}

impl LegacyMarker {
    fn write(&self, marker: &PendingPush) -> Result<(), String> {
        AtomicWrite::write(
            self.store.as_ref(),
            &self.path,
            pending_push::render(marker).as_bytes(),
        )
        .map_err(|error| error.to_string())
    }

    /// Spending a marker is best effort: a marker no issue backs is
    /// recognised and cleared by the next push of the same title.
    fn spend(&self) {
        if let Err(error) = self.store.remove(&self.path) {
            tracing::warn!(
                marker = %self.path.display(),
                %error,
                "could not remove a spent pending-push marker"
            );
        }
    }
}

fn push_legacy_item(
    creation: &Creation<'_>,
    body: &str,
    pushing: &LegacyPushing<'_>,
) -> Result<LegacyPush, String> {
    let item = creation.item;
    let state = pushing.state;
    let marker_path =
        pending_push::path(&state.root, &state.name, &item.slug());
    let marker_content = creation.reader.read(&marker_path).ok().flatten();
    let parsed = pending_push::read(marker_content.as_deref());
    let digest = pending_push::request_digest(item.title(), body, item.kind());
    let marker_state = match &parsed {
        Err(_) => MarkerState::Unreadable,
        Ok(None) => MarkerState::Absent,
        Ok(Some(marker)) => MarkerState::Present(marker),
    };
    let corpus = creation.identities()?;
    let corpus_carries = |id: &ExternalId| carries_external_id(&corpus, id);
    let precondition =
        work::sync::push_precondition(&marker_state, &digest, &corpus_carries);

    match precondition {
        PushPrecondition::Refuse(reason) => Err(refusal_message(
            reason,
            &marker_path,
            parsed.ok().flatten().as_ref(),
        )),
        PushPrecondition::ReuseId(key) => Ok(LegacyPush {
            outcome: PushOutcome::WriteOnce,
            key: Some(key),
            cause: None,
            tracker: (pushing.tracker)().ok(),
            marker: Some(marker_path),
        }),
        PushPrecondition::Proceed => {
            pending_push::prepare_dir(&state.root, &state.name)
                .map_err(|error| error.to_string())?;
            let tracker = match (pushing.tracker)() {
                Ok(tracker) => tracker,
                Err(outcome) => {
                    return Ok(LegacyPush::without_issue(outcome, None));
                }
            };
            let fingerprint = RequestFingerprint {
                title: item.title().to_owned(),
                digest,
                attempted_at: creation.now,
                failure: None,
            };
            let marker = LegacyMarker {
                store: (creation.store_at)(
                    marker_path.parent().unwrap_or(&state.root),
                ),
                path: marker_path,
            };
            marker.write(&PendingPush::Attempted {
                request: fingerprint.clone(),
            })?;
            send_legacy_create(
                &CreateRequest {
                    title: item.title(),
                    body,
                    kind: item.kind(),
                },
                tracker,
                &marker,
                fingerprint,
            )
        }
    }
}

/// Sends the create and records its outcome in the marker: the key once
/// created, the failure when the outcome is unknown, and nothing once no
/// issue can exist.
fn send_legacy_create(
    request: &CreateRequest<'_>,
    tracker: Box<dyn RemoteTracker>,
    marker: &LegacyMarker,
    fingerprint: RequestFingerprint,
) -> Result<LegacyPush, String> {
    match send_create(request, tracker.as_ref()) {
        RemoteCreate::Created(key) => {
            marker.write(&PendingPush::Created {
                request: fingerprint,
                external_id: key.clone(),
            })?;
            Ok(LegacyPush {
                outcome: PushOutcome::WriteOnce,
                key: Some(key),
                cause: None,
                tracker: Some(tracker),
                marker: Some(marker.path.clone()),
            })
        }
        RemoteCreate::TrackerUnreachable => {
            marker.spend();
            Ok(LegacyPush::without_issue(PushOutcome::LocalSave, None))
        }
        RemoteCreate::Rejected { detail } => {
            marker.spend();
            Ok(LegacyPush::without_issue(
                PushOutcome::Rejected,
                Some(detail),
            ))
        }
        RemoteCreate::OutcomeUnknown { detail } => {
            marker.write(&PendingPush::Attempted {
                request: RequestFingerprint {
                    failure: Some(detail),
                    ..fingerprint
                },
            })?;
            Ok(LegacyPush::without_issue(PushOutcome::LoudTerminal, None))
        }
    }
}

fn read_back_of(tracker: &dyn RemoteTracker, key: &ExternalId) -> RemoteHash {
    tracker.show(key).map_or(RemoteHash::Unknown, |issue| {
        RemoteHash::Known(ReadBack {
            hash: digest::remote_body(&issue.body),
            updated: issue.updated,
        })
    })
}

/// Records what the item and its new issue look like now, so the next sync
/// classifies the pair as synced.
fn record_baseline_after_create(
    creation: &Creation<'_>,
    state: &IntegrationState,
    item_id: &str,
    written: &str,
    remote: RemoteHash,
) -> Result<(), String> {
    let path = state.baseline_path();
    let dir = path.parent().unwrap_or(creation.repo_root).to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let writer = (creation.store_at)(&dir);
    let store = BaselineStore::new(path, creation.reader, writer.as_ref());
    record_created_baseline(
        item_id,
        &IntendedBaseline {
            remote_hash: remote,
            local_hash: digest::local(written)
                .map_err(|error| error.to_string())?,
        },
        &store,
        creation.now,
    )
    .map_err(|error| error.to_string())
}

/// Writes a locally numbered item under the ID `allocate` gives it, pushing
/// it first when `pushing` names where to.
///
/// # Errors
///
/// A message when the ID cannot be allocated, the item cannot be written,
/// or an earlier push of the same title refuses this one.
pub fn create_local_item(
    creation: &Creation<'_>,
    allocate: &dyn Fn() -> Result<String, String>,
    pushing: Option<&LegacyPushing<'_>>,
) -> Result<CreationOutcome, String> {
    let _held = creation.hold_create_lock()?;
    let id = allocate()?;
    let target = creation.target_for(&id);
    creation.refuse_to_overwrite(&target)?;
    let store = creation.item_store();

    let Some(pushing) = pushing else {
        creation.write_new(
            store.as_ref(),
            &target,
            &creation.item.content(&id, None)?,
        )?;
        return Ok(CreationOutcome::unpushed(target));
    };
    let push = push_legacy_item(creation, &creation.item.body(&id)?, pushing)?;
    let item = creation
        .item
        .content(&id, push.key.as_ref().map(ExternalId::as_str))?;
    let written =
        creation.write_new_retrying_once(store.as_ref(), &target, &item);
    let Some(key) = push.key else {
        written?;
        return Ok(pushed(Some(target), push.outcome, None, push.cause));
    };
    if let Err(cause) = written {
        return Ok(pushed(
            None,
            PushOutcome::CreatedUnwritten,
            Some(&key),
            Some(cause),
        ));
    }
    let remote = push
        .tracker
        .as_deref()
        .map_or(RemoteHash::Unknown, |tracker| read_back_of(tracker, &key));
    record_baseline_after_create(creation, pushing.state, &id, &item, remote)?;
    if let Some(path) = push.marker {
        LegacyMarker {
            store: (creation.store_at)(
                path.parent().unwrap_or(&pushing.state.root),
            ),
            path,
        }
        .spend();
    }
    Ok(pushed(
        Some(target),
        PushOutcome::WriteOnce,
        Some(&key),
        None,
    ))
}

/// How an earlier create of the same content is still pending.
enum PendingCreate {
    Record {
        draft: DraftId,
        stage: PromotionStage,
        draft_path: Option<PathBuf>,
    },
    UnreadableRecord {
        path: PathBuf,
        draft_path: PathBuf,
    },
    LegacyMarker(PathBuf),
    Draft(PathBuf),
}

impl PendingCreate {
    /// The draft the earlier create left, where one still exists.
    fn existing_draft(&self) -> Option<PathBuf> {
        match self {
            Self::Record { draft_path, .. } => draft_path.clone(),
            Self::UnreadableRecord { draft_path, .. }
            | Self::Draft(draft_path) => Some(draft_path.clone()),
            Self::LegacyMarker(_) => None,
        }
    }

    fn message(&self, title: &str) -> String {
        match self {
            Self::Record {
                draft,
                stage: PromotionStage::Attempted,
                ..
            } => format!(
                "E_PUSH_PENDING: {draft} records a create of this content whose \
                 outcome is unknown; if the tracker has its issue run `work \
                 promote {draft} --adopt <KEY>`, otherwise `work promote \
                 {draft} --create`",
                draft = draft.as_str()
            ),
            Self::Record { draft, .. } => format!(
                "E_PUSH_PENDING: {draft} is already being promoted onto its \
                 issue; run `work promote {draft}` or `work sync` to finish it",
                draft = draft.as_str()
            ),
            Self::UnreadableRecord { path, .. } => format!(
                "E_PUSH_PENDING: {} could not be read and may record a create \
                 of this content; inspect it before creating again",
                path.display()
            ),
            Self::LegacyMarker(path) => format!(
                "E_PUSH_PENDING: {} records an earlier create titled '{title}'; \
                 check the tracker for an issue titled '{title}' first, then \
                 inspect or remove the named marker",
                path.display()
            ),
            Self::Draft(path) => format!(
                "E_DRAFT_EXISTS: {} already holds this content; run `work \
                 promote <draft>` or `work sync`",
                path.display()
            ),
        }
    }
}

/// The draft's own content digest: the same content digests alike
/// whatever draft ID each copy was given.
fn draft_content_digest(content: &str, id: &str) -> Option<String> {
    let (_, body) = digest::split_frontmatter_and_body(content).ok()?;
    let fields = create_request_fields::read(content).ok()?;
    Some(pending_push::content_digest(
        &fields.title,
        &body,
        &fields.kind,
        id,
    ))
}

/// Finds an earlier create of the same content still pending, so a rerun
/// after a tracker error, a kill or an unwritten create never sends a
/// second create.
fn pending_create(
    creation: &Creation<'_>,
    records: &dyn PromotionRecords,
    state: &IntegrationState,
) -> Result<Option<PendingCreate>, String> {
    let item = creation.item;
    let (_, placeholder_body) = digest::split_frontmatter_and_body(
        &item.content(ID_PLACEHOLDER, None)?,
    )
    .map_err(|error| error.to_string())?;
    let wanted = pending_push::content_digest(
        item.title(),
        &placeholder_body,
        item.kind(),
        ID_PLACEHOLDER,
    );
    let files = FilesystemWorkItemFiles::new(creation.work_dir)
        .files()
        .map_err(|error| error.to_string())?;
    let drafts: Vec<(DraftId, &Path, &str)> = files
        .iter()
        .filter_map(|file| {
            let identity = work::work_item_files::identity_of(file)?;
            let draft = DraftId::parse(&identity.id)?;
            Some((draft, file.path.as_path(), file.content.as_str()))
        })
        .collect();
    let draft_path = |wanted: &DraftId| {
        drafts
            .iter()
            .find(|(draft, _, _)| draft == wanted)
            .map(|(_, path, _)| path.to_path_buf())
    };

    for entry in records.outstanding().map_err(|error| error.to_string())? {
        match entry {
            Ok(record) if record.content_digest == wanted => {
                return Ok(Some(PendingCreate::Record {
                    draft_path: draft_path(&record.draft_id),
                    draft: record.draft_id,
                    stage: record.stage,
                }));
            }
            Ok(_) => {}
            Err(unreadable) => {
                let live_draft = unreadable
                    .path
                    .file_stem()
                    .and_then(std::ffi::OsStr::to_str)
                    .and_then(DraftId::parse)
                    .and_then(|draft| draft_path(&draft));
                if let Some(draft_path) = live_draft {
                    return Ok(Some(PendingCreate::UnreadableRecord {
                        path: unreadable.path,
                        draft_path,
                    }));
                }
            }
        }
    }
    let markers = pending_push::outstanding(&state.root, &state.name)
        .map_err(|error| error.to_string())?;
    for (path, marker) in markers.into_iter().flatten() {
        if let Marker::Legacy(
            PendingPush::Attempted { request }
            | PendingPush::Created { request, .. },
        ) = marker
        {
            if request.title == item.title() {
                return Ok(Some(PendingCreate::LegacyMarker(path)));
            }
        }
    }
    Ok(drafts
        .iter()
        .find(|(draft, _, content)| {
            draft_content_digest(content, draft.as_str()).as_deref()
                == Some(wanted.as_str())
        })
        .map(|(_, path, _)| PendingCreate::Draft(path.to_path_buf())))
}

/// What the user does to unblock a draft whose retirement onto `key` was
/// refused.
#[must_use]
pub fn blocked_remedy(
    refusal: &RetirementRefusal,
    key: &ExternalId,
    draft: &DraftId,
) -> String {
    let draft = draft.as_str();
    match refusal {
        RetirementRefusal::IdTaken { holder, .. }
        | RetirementRefusal::KeyLinked { holder } => format!(
            "{holder} already carries {key}; if it is the same issue, delete \
             the draft or merge it into {holder}, otherwise fix {holder}, \
             then `work promote {draft}`",
            holder = holder.display()
        ),
        RetirementRefusal::TargetExists(path) => format!(
            "move {} aside, then `work promote {draft}`",
            path.display()
        ),
        RetirementRefusal::ItemNotFound(_) => {
            format!("the draft {draft} is gone; the issue {key} is unlinked")
        }
    }
}

/// A retirement that could not restore every path: the path line names
/// whichever of the draft and the target exists, and stderr the paths to
/// restore.
fn incomplete_outcome(
    failure: &RetirementFailure,
    creation: &Creation<'_>,
    draft: &DraftId,
    draft_path: &Path,
    key: Option<&ExternalId>,
) -> CreationOutcome {
    let target = key.map(|key| creation.target_for(key.as_str()));
    let path = std::iter::once(draft_path)
        .chain(target.as_deref())
        .find(|path| creation.exists(path))
        .map(Path::to_path_buf);
    let cause =
        key.and_then(|key| TrackerKey::parse(key.as_str()))
            .map(|key| {
                let retirement = Retirement::onto(draft.as_str(), &key);
                failure.message(
                    &retirement,
                    &creation.state_dir.join(retirement.recovery_dir()),
                )
            });
    pushed(path, PushOutcome::RetirementIncomplete, key, cause)
}

/// Maps a promotion's row onto the create outcome the skills read.
fn promotion_outcome(
    row: PromotionRow,
    creation: &Creation<'_>,
    draft: &DraftId,
) -> CreationOutcome {
    let (reason, key) = match row.outcome {
        PromotionOutcome::Promoted(
            Promotion::Completed(key, _) | Promotion::AlreadyDone(key),
        ) => {
            let path = creation
                .identities()
                .ok()
                .and_then(|items| {
                    items
                        .into_iter()
                        .find(|item| item.id.eq_ignore_ascii_case(key.as_str()))
                })
                .map_or_else(
                    || creation.target_for(key.as_str()),
                    |item| item.path,
                );
            return pushed(
                Some(path),
                PushOutcome::WriteOnce,
                Some(&key),
                None,
            );
        }
        PromotionOutcome::Previewed => {
            unreachable!("a create never previews its promotion")
        }
        PromotionOutcome::NotPromoted { reason, held_key } => {
            (reason, held_key)
        }
    };
    let draft_path = row.path;
    let outcome = reason.create_outcome();
    let shown_key = match outcome {
        PushOutcome::CreatedUnwritten | PushOutcome::CreatedBlocked => {
            key.as_ref()
        }
        _ => None,
    };
    let mut created = match reason {
        NotPromoted::RetirementFailed(
            failure @ RetirementFailure::RestoreIncomplete { .. },
        ) => incomplete_outcome(
            &failure,
            creation,
            draft,
            &draft_path,
            key.as_ref(),
        ),
        NotPromoted::Refused(RetirementRefusal::ItemNotFound(_)) => {
            pushed(None, outcome, shown_key, None)
        }
        NotPromoted::Refused(refusal) => {
            let remedy =
                key.as_ref().map(|key| blocked_remedy(&refusal, key, draft));
            pushed(Some(draft_path), outcome, shown_key, remedy)
        }
        NotPromoted::RequestRejected { detail }
        | NotPromoted::RecordUnwritable { detail, .. } => {
            pushed(Some(draft_path), outcome, shown_key, Some(detail))
        }
        _ => pushed(Some(draft_path), outcome, shown_key, None),
    };
    if let Some(report) = created.push.as_mut() {
        if matches!(
            report.outcome,
            PushOutcome::CreatedBlocked | PushOutcome::RetirementIncomplete
        ) {
            report.details = row.details;
        }
    }
    created
}

/// Under `{tracker}` with a push: writes a draft, then has `promotion`
/// promote it, so an interrupted create always leaves a draft its record
/// belongs to.
///
/// # Errors
///
/// [`CreateFailure::Pending`] when an earlier create of the same content
/// is still pending, otherwise [`CreateFailure::Failed`].
pub fn create_tracker_keyed_item(
    creation: &Creation<'_>,
    state: &IntegrationState,
    records: &dyn PromotionRecords,
    draws: &mut dyn SuffixDraws,
    drafted: &mut Drafted<'_>,
    promotion: &mut DraftPromotion<'_>,
) -> Result<CreationOutcome, CreateFailure> {
    std::fs::create_dir_all(&state.root).map_err(|error| error.to_string())?;
    let held = creation.hold_create_lock()?;
    if let Some(pending) = pending_create(creation, records, state)? {
        return Err(CreateFailure::Pending {
            message: pending.message(creation.item.title()),
            existing_draft: pending.existing_draft(),
        });
    }
    let (draft, draft_path) = creation.write_draft(draws, &held)?;
    drop(held);
    if let Err(cause) = drafted(&draft) {
        return Ok(pushed(
            Some(draft_path),
            PushOutcome::LocalSave,
            None,
            Some(cause),
        ));
    }
    let promoted = promotion(&draft, draft_path.clone())?;
    Ok(promoted.map_or_else(
        || pushed(Some(draft_path), PushOutcome::LocalSave, None, None),
        |row| promotion_outcome(row, creation, &draft),
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use tracker::ExternalId;

    use super::carries_external_id;
    use super::corpus_identities;

    #[test]
    fn the_push_duplicate_check_sees_external_ids_in_drafts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let drafts = dir.path().join("drafts");
        std::fs::create_dir_all(&drafts).expect("drafts dir");
        std::fs::write(
            drafts.join("draft-k7mq3x-title.md"),
            "---\nid: \"draft-k7mq3x\"\nexternal_id: \"ENG-42\"\n---\n",
        )
        .expect("write draft");

        let corpus = corpus_identities(dir.path()).expect("listable corpus");
        let carries = |key: &str| {
            carries_external_id(&corpus, &ExternalId::new(key.to_owned()))
        };

        assert!(carries("ENG-42"));
        assert!(carries("eng-42"));
        assert!(!carries("ENG-43"));
    }
}
