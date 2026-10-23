//! The per-item apply sequence: push, pull and finalise, over the frozen
//! `tracker::RemoteTracker` port and a `BaselineStore`.

use std::path::Path;
use std::path::PathBuf;

use corpus::body::BodyParser;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::Located;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker::Unrenderable;
use vcs::tracking::FileTracking;
use work::inventory::Fault;
use work::sync::order_precondition;
use work::sync::push_precondition;
use work::sync::BodyEquivalence;
use work::sync::Dirtiness;
use work::sync::DualRecipeFingerprints;
use work::sync::MarkerState;
use work::sync::OrderPrecondition;
use work::sync::PendingPush;
use work::sync::PendingPushOrder;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;
use work::tracker_key::TrackerKey;

pub use crate::sync::attempt::Attempted;
pub use crate::sync::attempt::ItemProgress;
pub use crate::sync::attempt::Landing;

use crate::sync::attempt::ItemAttempt;
use crate::sync::attempt::PublishError;
use crate::sync::attempt::StepError;
use crate::sync::baseline::Entry;
use crate::sync::baseline::SyncStamps;
use crate::sync::baseline_digester::BaselineDigester;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::body_comparison::BodyComparison;
use crate::sync::create::canonical_external_key;
use crate::sync::create::DiscoveredIssue;
use crate::sync::create::LocalAuthor;
use crate::sync::digest;
use crate::sync::fetch::WorkingCopyStatus;
use crate::sync::files::SyncFiles;
use crate::sync::operation::SyncOperation;
use crate::sync::pending_push;
use crate::sync::recovery_label::RecoveryLabel;
use crate::sync::report::Adoption;
use crate::sync::report::CreateNotice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    Terminal,
    Unconfigured,
    Rejected,
    Unrenderable,
    PullResidue,
    Unparseable,
    WriteRefused,
    OrderRefused,
}

impl FailureClass {
    /// How a failed row names this class.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Retryable => "retryable",
            Self::Terminal => "terminal",
            Self::Unconfigured => "unconfigured",
            Self::Rejected => "rejected",
            Self::Unrenderable => "unrenderable",
            Self::PullResidue => "pull-residue",
            Self::Unparseable => "unparseable",
            Self::WriteRefused => "local-write-refused",
            Self::OrderRefused => "order-refused",
        }
    }
}

#[derive(Debug)]
pub enum ApplyError {
    Tracker {
        item_id: String,
        operation: SyncOperation,
        source: TrackerError,
    },
    Io {
        item_id: String,
        operation: SyncOperation,
        progress: ItemProgress,
        detail: String,
    },
    RemoteBodyUnavailable {
        item_id: String,
        operation: SyncOperation,
    },
    RemoteUnrenderable {
        item_id: String,
        operation: SyncOperation,
        cause: Unrenderable,
    },
    LocalPullResidue {
        item_id: String,
        operation: SyncOperation,
        external_id: ExternalId,
    },
    /// The file's content is wrong: not UTF-8 (`read-local`), or
    /// frontmatter that does not parse (`parse-local`).
    LocalUnparseable {
        item_id: String,
        operation: SyncOperation,
        detail: String,
    },
    LocalWriteRefused {
        item_id: String,
        detail: String,
    },
    LocalChangedSinceClassified {
        item_id: String,
    },
    /// A pending-push marker refused the create: an earlier attempt may
    /// have made an issue, so a person must look before anything is sent.
    PendingPushRefused {
        item_id: String,
        reason: RefusalReason,
        detail: String,
    },
    /// A pending-push order the run would not carry out: the request itself
    /// must change.
    OrderRefused {
        item_id: String,
        order: PendingPushOrder,
        refusal: Box<OrderRefusal>,
    },
    /// Deferred because an unreadable or malformed work file may carry the
    /// key the step needs to know is carried nowhere else.
    CorpusIncomplete {
        subject: String,
        deferred: IncompleteCorpusDeferral,
        blocking: Fault,
    },
}

/// Why a pending-push order was not carried out; each cause ends in the
/// next step a person takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderRefusal {
    PullOnly,
    Draft {
        id: String,
    },
    ConfirmedLink {
        id: String,
        key: ExternalId,
        path: PathBuf,
    },
    TargetedOut {
        id: String,
    },
    NoSuchItem {
        id: String,
    },
    DeferredByIdentity {
        id: String,
    },
    IssueNotFound {
        key: ExternalId,
    },
    IssueMoved {
        id: String,
        key: ExternalId,
        to: TrackerKey,
    },
    MovedToUnknownKey {
        key: ExternalId,
    },
    KeyCarried {
        key: ExternalId,
        carrier: KeyCarrier,
    },
    MarkerRecords {
        id: String,
        key: ExternalId,
    },
    AlreadyWritten {
        carrier: KeyCarrier,
    },
}

impl std::fmt::Display for OrderRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PullOnly => formatter.write_str(
                "orders settle pending pushes; run without --pull-only",
            ),
            Self::Draft { id } => write!(
                formatter,
                "{id} is a draft; promote it with accelerator work promote \
                 {id}"
            ),
            Self::ConfirmedLink { id, key, path } => write!(
                formatter,
                "{id} already carries external_id {key}; if {key} is the \
                 wrong issue, check which is right in the tracker and edit \
                 external_id in {} by hand",
                path.display()
            ),
            Self::TargetedOut { id } => {
                write!(formatter, "{id} is not in this run's --target")
            }
            Self::NoSuchItem { id } => {
                write!(formatter, "no work item {id} in this run")
            }
            Self::DeferredByIdentity { id } => write!(
                formatter,
                "{id} was deferred by identity settlement; settle its \
                 identity row first"
            ),
            Self::IssueNotFound { key } => {
                write!(formatter, "issue {key} not found; check the key")
            }
            Self::IssueMoved { id, key, to } => write!(
                formatter,
                "issue {key} moved to {to}; run --resolve {id}=adopt:{to}"
            ),
            Self::MovedToUnknownKey { key } => write!(
                formatter,
                "issue {key} moved to a key this tracker does not use; check \
                 the key in the tracker"
            ),
            Self::KeyCarried { key, carrier } => {
                write!(formatter, "{key} is already carried by {carrier}")
            }
            Self::MarkerRecords { id, key } => write!(
                formatter,
                "marker records {key}; run --resolve {id}=adopt:{key}"
            ),
            Self::AlreadyWritten { carrier } => write!(
                formatter,
                "already written by {carrier}; decide which item is the real \
                 one"
            ),
        }
    }
}

/// What a run defers while its corpus is incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompleteCorpusDeferral {
    Import(Vec<ExternalId>),
    ReuseId(ExternalId),
    Adopt(ExternalId),
}

impl ApplyError {
    /// A local failure is retryable when nothing for the item had landed,
    /// since a re-run cannot double-apply, and terminal once something may
    /// have. A tracker failure keeps the tracker's class: a terminal one
    /// already means its mutation may have landed.
    #[must_use]
    pub const fn class(&self) -> FailureClass {
        match self {
            Self::Tracker { source, .. } => match source {
                TrackerError::Retryable { .. } => FailureClass::Retryable,
                TrackerError::Terminal { .. } => FailureClass::Terminal,
                TrackerError::Unconfigured { .. } => FailureClass::Unconfigured,
                TrackerError::Rejected { .. } => FailureClass::Rejected,
            },
            Self::Io {
                progress: ItemProgress::NothingLanded,
                ..
            }
            | Self::RemoteBodyUnavailable { .. }
            | Self::LocalChangedSinceClassified { .. }
            | Self::CorpusIncomplete {
                blocking: Fault::Environment,
                ..
            } => FailureClass::Retryable,
            Self::Io {
                progress: ItemProgress::MayHaveLanded,
                ..
            }
            | Self::PendingPushRefused { .. } => FailureClass::Terminal,
            Self::RemoteUnrenderable { .. } => FailureClass::Unrenderable,
            Self::LocalPullResidue { .. } => FailureClass::PullResidue,
            Self::LocalUnparseable { .. }
            | Self::CorpusIncomplete {
                blocking: Fault::Content,
                ..
            } => FailureClass::Unparseable,
            Self::LocalWriteRefused { .. } => FailureClass::WriteRefused,
            Self::OrderRefused { .. } => FailureClass::OrderRefused,
        }
    }
}

impl ApplyError {
    #[must_use]
    pub fn item_id(&self) -> &str {
        match self {
            Self::Tracker { item_id, .. }
            | Self::Io { item_id, .. }
            | Self::RemoteBodyUnavailable { item_id, .. }
            | Self::RemoteUnrenderable { item_id, .. }
            | Self::LocalPullResidue { item_id, .. }
            | Self::LocalUnparseable { item_id, .. }
            | Self::LocalWriteRefused { item_id, .. }
            | Self::LocalChangedSinceClassified { item_id }
            | Self::PendingPushRefused { item_id, .. }
            | Self::OrderRefused { item_id, .. }
            | Self::CorpusIncomplete {
                subject: item_id, ..
            } => item_id,
        }
    }

    #[must_use]
    pub const fn operation(&self) -> SyncOperation {
        match self {
            Self::Tracker { operation, .. }
            | Self::Io { operation, .. }
            | Self::RemoteBodyUnavailable { operation, .. }
            | Self::RemoteUnrenderable { operation, .. }
            | Self::LocalPullResidue { operation, .. }
            | Self::LocalUnparseable { operation, .. } => *operation,
            Self::LocalWriteRefused { .. } => SyncOperation::WriteLocal,
            Self::LocalChangedSinceClassified { .. } => {
                SyncOperation::RecheckLocal
            }
            Self::PendingPushRefused { .. } => SyncOperation::Create,
            Self::OrderRefused { order, .. } => match order {
                PendingPushOrder::Adopt(_) => SyncOperation::Adopt,
                PendingPushOrder::Create => SyncOperation::Create,
            },
            Self::CorpusIncomplete { deferred, .. } => match deferred {
                IncompleteCorpusDeferral::Import(_) => SyncOperation::Discover,
                IncompleteCorpusDeferral::ReuseId(_) => {
                    SyncOperation::LinkExternalId
                }
                IncompleteCorpusDeferral::Adopt(_) => SyncOperation::Adopt,
            },
        }
    }

    /// Why the operation failed, naming neither the item nor the operation.
    #[must_use]
    pub fn cause(&self) -> String {
        match self {
            Self::Tracker { source, .. } => source.to_string(),
            Self::Io { detail, .. } => detail.clone(),
            Self::RemoteBodyUnavailable { .. } => {
                "the remote description was not fetched this run, so nothing \
                 was sent or written; re-run the sync"
                    .to_owned()
            }
            Self::RemoteUnrenderable { cause, .. } => cause.to_string(),
            Self::LocalPullResidue {
                item_id,
                external_id,
                ..
            } => format!(
                "the local file still holds the raw tracker document an \
                 earlier pull wrote, so nothing was sent; replace it with \
                 Markdown (`jira show {external_id} --render-adf` prints a \
                 starting point) and sync, or resolve with --resolve \
                 {item_id}=remote to discard local changes"
            ),
            Self::LocalUnparseable {
                operation: SyncOperation::ParseLocal,
                detail,
                ..
            } => format!(
                "the local file's frontmatter does not parse ({detail}), so \
                 nothing was sent or written; fix the file and re-run the sync"
            ),
            Self::LocalUnparseable { detail, .. } => format!(
                "the local file cannot be read as text ({detail}), so nothing \
                 was sent or written; fix the file and re-run the sync"
            ),
            Self::LocalWriteRefused { detail, .. } => format!(
                "the local file was not written ({detail}), so it is as it \
                 was; the same write is refused until that changes"
            ),
            Self::LocalChangedSinceClassified { .. } => {
                "the local file changed after the run read it, so nothing was \
                 written or recorded; re-run the sync"
                    .to_owned()
            }
            Self::PendingPushRefused {
                item_id,
                reason,
                detail,
            } => {
                let remedy = if *reason == RefusalReason::AlreadyWritten {
                    String::new()
                } else {
                    format!(
                        "; search the tracker for this item, then settle with \
                         --resolve {item_id}=adopt:<KEY> if it exists or \
                         --resolve {item_id}=create if it does not"
                    )
                };
                format!(
                    "pending push refused ({}): {detail}{remedy}",
                    reason.token()
                )
            }
            Self::OrderRefused { refusal, .. } => refusal.to_string(),
            Self::CorpusIncomplete { deferred, .. } => {
                let unknown = match deferred {
                    IncompleteCorpusDeferral::Import(keys) => format!(
                        "{} cannot be shown to be untracked",
                        keys.iter()
                            .map(ExternalId::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    IncompleteCorpusDeferral::ReuseId(key) => format!(
                        "{}, which the pending-push marker records, cannot \
                         be shown to be carried by no other work item",
                        key.as_str()
                    ),
                    IncompleteCorpusDeferral::Adopt(key) => format!(
                        "{}, which --resolve adopts, cannot be shown to be \
                         carried by no other work item",
                        key.as_str()
                    ),
                };
                format!(
                    "the work directory holds unreadable or malformed files, \
                     so {unknown}"
                )
            }
        }
    }
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {} failed: {}",
            self.item_id(),
            self.operation(),
            self.cause()
        )
    }
}

impl std::error::Error for ApplyError {}

/// A send of the local body; with no `title`, the remote title is left as
/// it is. `sent_content` is the local file the title and body came from,
/// which the baseline records.
pub struct PushRequest<'a> {
    pub id: &'a str,
    pub external_id: &'a ExternalId,
    pub title: Option<&'a str>,
    pub body: &'a str,
    pub sent_content: &'a str,
}

/// A send of the local title alone, leaving the remote description as it is.
/// `sent_content` is the local file the title came from, which the baseline
/// records.
pub struct RetitleRequest<'a> {
    pub id: &'a str,
    pub external_id: &'a ExternalId,
    pub title: &'a str,
    pub sent_content: &'a str,
}

/// Who already carries a tracker key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCarrier {
    WorkItem(String),
    FollowedThisRun,
}

impl std::fmt::Display for KeyCarrier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkItem(id) => write!(formatter, "work item {id}"),
            Self::FollowedThisRun => {
                formatter.write_str("this run's identity settlement")
            }
        }
    }
}

/// The inputs `create_from_local` needs beyond the applier's own ports.
///
/// `marker_path` is precomputed by the run so the applier stays ignorant of
/// the integrations layout, and `carrier_of` names whoever already carries a
/// candidate `external_id` — the guard that stops a recovered `Created`
/// marker binding two files to one remote issue.
pub struct CreateFromLocalRequest<'a> {
    pub item_id: &'a str,
    pub file_path: &'a Path,
    pub title: &'a str,
    pub body: &'a str,
    pub kind: &'a str,
    pub marker_path: &'a Path,
    pub author: &'a dyn LocalAuthor,
    pub carrier_of: &'a dyn Fn(&ExternalId) -> Option<KeyCarrier>,
    /// What keeps the corpus incomplete, if anything: a marker's key cannot
    /// be linked while an unusable file may already carry it.
    pub corpus_blocked_by: Option<Fault>,
    /// How a person settled this item's refused pending push, if they did.
    pub order: Option<&'a PendingPushOrder>,
    /// The active tracker's name, for causes that name its keys.
    pub tracker_name: &'a str,
    pub attempted_at: u64,
}

/// A local create that applied, with what a person should know of it.
#[derive(Debug, Default)]
pub struct CreatedLocally {
    pub leftover_marker: Option<LeftoverMarker>,
    pub notice: Option<CreateNotice>,
    pub adopted: Option<Adoption>,
}

impl From<Option<LeftoverMarker>> for CreatedLocally {
    fn from(leftover_marker: Option<LeftoverMarker>) -> Self {
        Self {
            leftover_marker,
            notice: None,
            adopted: None,
        }
    }
}

/// What a refused marker shows, without a remedy: the person decides what
/// to do once they have looked.
fn refused_marker_state(
    reason: RefusalReason,
    marker: Option<&PendingPush>,
    carrier: Option<KeyCarrier>,
) -> String {
    let recorded_key = match marker {
        Some(PendingPush::Created { external_id, .. }) => {
            Some(external_id.as_str())
        }
        _ => None,
    };
    match (reason, marker) {
        (RefusalReason::MarkerUnreadable, _) => {
            "the marker could not be parsed".to_owned()
        }
        (
            RefusalReason::PriorAttemptUnknownOutcome,
            Some(PendingPush::Attempted { request }),
        ) => format!(
            "an earlier create attempted at {} has an unknown outcome{}",
            request.attempted_at,
            request
                .failure
                .as_deref()
                .map(|failure| format!(" (failure: {failure})"))
                .unwrap_or_default()
        ),
        (RefusalReason::PriorAttemptUnknownOutcome, _) => {
            "an earlier create has an unknown outcome".to_owned()
        }
        (RefusalReason::FingerprintMismatch, _) => format!(
            "the marker records {} for a different request",
            recorded_key.unwrap_or("an issue")
        ),
        (RefusalReason::AlreadyWritten, _) => {
            let key = recorded_key.unwrap_or("its key");
            match carrier {
                Some(KeyCarrier::FollowedThisRun) => format!(
                    "{key} is already followed by this run's identity \
                     settlement"
                ),
                Some(KeyCarrier::WorkItem(id)) => {
                    format!("{key} is already carried by work item {id}")
                }
                None => format!("{key} is already carried"),
            }
        }
    }
}

/// A pending-push marker as a create may rely on it. A distrusted marker
/// still stops a create, but never steers an adopt.
enum MarkerReading {
    Absent,
    Unreadable,
    Distrusted {
        cause: String,
        /// The issue it records as created, when that is a key at all.
        created: Option<ExternalId>,
    },
    Trusted(PendingPush),
}

impl MarkerReading {
    const fn state(&self) -> MarkerState<'_> {
        match self {
            Self::Absent => MarkerState::Absent,
            Self::Unreadable | Self::Distrusted { .. } => {
                MarkerState::Unreadable
            }
            Self::Trusted(marker) => MarkerState::Present(marker),
        }
    }

    const fn trusted(&self) -> Option<&PendingPush> {
        match self {
            Self::Trusted(marker) => Some(marker),
            _ => None,
        }
    }

    fn distrust(&self) -> Option<&str> {
        match self {
            Self::Distrusted { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

/// A synced item's baseline, rewritten under the current recipe from the
/// fingerprints classification read.
pub struct RecordRequest<'a> {
    pub id: &'a str,
    pub file_path: &'a Path,
    pub fingerprints: DualRecipeFingerprints,
    pub remote_updated: RemoteTimestamp,
}

pub struct PullRequest<'a> {
    pub id: &'a str,
    pub file_path: &'a Path,
    /// The local file the pull was classified against, which only it may
    /// overwrite.
    pub classified_content: &'a str,
    pub content: &'a str,
    /// Un-normalised, and hashed here rather than by the caller, so the
    /// baseline's `remote_hash` always comes from the projection actually
    /// written.
    pub projected_body: &'a str,
    pub remote_updated: RemoteTimestamp,
    pub unchanged_since_sync: UnchangedSinceSync,
    /// Where the local file's edits are kept should the pull need to keep
    /// them; `None` when the id cannot name a directory.
    pub recovery_label: Option<RecoveryLabel>,
}

/// Whether the local file still digests to what its baseline entry
/// recorded at the last sync, and under which recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnchangedSinceSync {
    RecipeOne,
    Legacy,
    No,
}

/// A pending-push marker left behind after its item was linked, which
/// never creates again, so the marker is harmless but should be removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftoverMarker {
    pub path: PathBuf,
    pub reason: String,
}

/// A store refusal a retry would repeat leaves the file as it was and keeps
/// its own class; a write that published before failing may have landed.
fn item_write_error(id: &str, error: StoreError) -> PublishError<StoreError> {
    match error {
        StoreError::UnsafePath { .. }
        | StoreError::AlreadyExists { .. }
        | StoreError::Validation { .. } => PublishError::Step(
            StepError::Refused(ApplyError::LocalWriteRefused {
                item_id: id.to_owned(),
                detail: error.to_string(),
            }),
        ),
        StoreError::NotDurable { .. } => {
            PublishError::FailedAfterPublish(error)
        }
        _ => PublishError::Step(StepError::Failed(error)),
    }
}

fn marker_write_error(path: &Path, error: &StoreError) -> PublishError<String> {
    let cause = format!("marker {}: {error}", path.display());
    match error {
        StoreError::NotDurable { .. } => {
            PublishError::FailedAfterPublish(cause)
        }
        _ => PublishError::Step(StepError::Failed(cause)),
    }
}

/// The ports an [`ItemApplier`] applies through.
#[derive(Clone, Copy)]
pub struct ApplierPorts<'a> {
    pub tracker: &'a dyn RemoteTracker,
    pub parser: &'a dyn BodyParser,
    pub status: &'a dyn WorkingCopyStatus,
    pub files: SyncFiles<'a>,
}

/// Applies one planned action, over the tracker port and a baseline store.
///
/// The store's borrow (`'store`) stays separate from the collaborators'
/// (`'ctx`) because a single lifetime is invariant: `&'a mut
/// BaselineStore<'a>` would borrow the store for the whole of the applier's
/// own lifetime, leaving the baseline unreadable afterwards.
///
/// Each public apply method begins an [`ItemAttempt`] and finishes it
/// around a private `*_steps` body, so an early return can never drop what
/// the attempt learnt.
pub struct ItemApplier<'ctx, 'store> {
    tracker: &'ctx dyn RemoteTracker,
    parser: &'ctx dyn BodyParser,
    status: &'ctx dyn WorkingCopyStatus,
    files: SyncFiles<'ctx>,
    baseline: &'store mut BaselineStore<'ctx>,
    run_start_epoch: u64,
}

impl<'ctx, 'store> ItemApplier<'ctx, 'store> {
    pub fn new(
        ports: ApplierPorts<'ctx>,
        baseline: &'store mut BaselineStore<'ctx>,
        run_start_epoch: u64,
    ) -> Self {
        Self {
            tracker: ports.tracker,
            parser: ports.parser,
            status: ports.status,
            files: ports.files,
            baseline,
            run_start_epoch,
        }
    }

    /// The baseline entry is written last, so a failed `update` leaves it
    /// unset and the next run reclassifies from scratch. It records the
    /// content sent, so an edit made since reads as a local change next run.
    /// A failed post-push
    /// `show` still writes the entry, with both remote fields empty and
    /// [`RemoteTimestamp::NotRead`] — the only place that variant is
    /// written.
    pub fn push(&mut self, request: &PushRequest<'_>) -> Attempted<()> {
        let mut attempt = ItemAttempt::begin(request.id);
        let result = self.push_steps(request, &mut attempt);
        attempt.finish(result)
    }

    fn push_steps(
        &self,
        request: &PushRequest<'_>,
        attempt: &mut ItemAttempt,
    ) -> Result<(), ApplyError> {
        attempt.land_tracker(
            SyncOperation::Update,
            Some(Landing::Push),
            || {
                self.tracker.update(
                    request.external_id,
                    request.title,
                    request.body,
                )
            },
        )?;
        self.record_read_back(
            attempt,
            request.external_id,
            request.sent_content,
        )
    }

    /// As [`Self::push`], sending the title alone; the local file is left
    /// as it is.
    pub fn retitle(&mut self, request: &RetitleRequest<'_>) -> Attempted<()> {
        let mut attempt = ItemAttempt::begin(request.id);
        let result = self.retitle_steps(request, &mut attempt);
        attempt.finish(result)
    }

    fn retitle_steps(
        &self,
        request: &RetitleRequest<'_>,
        attempt: &mut ItemAttempt,
    ) -> Result<(), ApplyError> {
        attempt.land_tracker(
            SyncOperation::Retitle,
            Some(Landing::Push),
            || self.tracker.retitle(request.external_id, request.title),
        )?;
        self.record_read_back(
            attempt,
            request.external_id,
            request.sent_content,
        )
    }

    fn record_read_back(
        &self,
        attempt: &ItemAttempt,
        external_id: &ExternalId,
        sent_content: &str,
    ) -> Result<(), ApplyError> {
        let remote = self.tracker.show(external_id).ok();
        self.record(attempt, sent_content, remote.as_ref())
    }

    /// A file already holding what the pull writes is written again, so a
    /// re-run after an interrupted baseline write still records it.
    ///
    /// The file is checked again after its edits are kept, because editors
    /// take no lock and keeping a durable copy takes long enough for one to
    /// write.
    ///
    /// The result fails with [`ApplyError::LocalChangedSinceClassified`]
    /// when the file holds neither the classified content nor what the pull
    /// writes, so an edit is never overwritten; otherwise with an
    /// [`ApplyError`] naming `request.id` and the operation attempted.
    pub fn pull(&mut self, request: &PullRequest<'_>) -> Attempted<()> {
        let mut attempt = ItemAttempt::begin(request.id);
        let result = self.pull_steps(request, &mut attempt);
        attempt.finish(result)
    }

    fn pull_steps(
        &self,
        request: &PullRequest<'_>,
        attempt: &mut ItemAttempt,
    ) -> Result<(), ApplyError> {
        let _held = attempt.observe_local(SyncOperation::LockLocal, || {
            self.lock_item(request)
        })?;
        let current = attempt
            .observe_local(SyncOperation::ReadLocal, || read_local(request))?;
        ensure_unchanged_since_classified(request, &current)?;
        let kept = attempt.observe_local(SyncOperation::KeepEdits, || {
            self.keep_edits_unless_synced(request, &current)
        })?;
        attempt.record_kept_edits(kept);
        let fingerprints = attempt
            .observe_local(SyncOperation::HashLocal, || {
                self.pulled_fingerprints(request)
            })?;
        let current = attempt
            .observe_local(SyncOperation::ReadLocal, || read_local(request))?;
        ensure_unchanged_since_classified(request, &current)?;
        attempt.land_local(
            SyncOperation::WriteLocal,
            Some(Landing::Pull),
            || self.write_item(request),
        )?;
        attempt.observe_local(SyncOperation::BaselineSet, || {
            self.set_pulled_baseline(request, fingerprints)
        })
    }

    /// Keeps a copy of `current` unless the last sync already captured it.
    ///
    /// A tracked file has history only once it is VCS-clean, so it is
    /// copied whenever it is not, even when the baseline matches. An
    /// untracked or ignored file reads as clean yet has no history, so only
    /// a recipe-1 baseline can vouch for it; a legacy entry cannot.
    fn keep_edits_unless_synced(
        &self,
        request: &PullRequest<'_>,
        current: &str,
    ) -> Result<Option<PathBuf>, StepError<String>> {
        if current == request.content || self.captured_by_last_sync(request) {
            return Ok(None);
        }
        let label = request.recovery_label.as_ref().ok_or_else(|| {
            StepError::Failed(
                "the id cannot name a recovery directory".to_owned(),
            )
        })?;
        let dir = label.dir();
        let recovery = self.files.recovery;
        recovery
            .prepare(&dir)
            .and_then(|()| {
                recovery.write_once(&dir, request.file_path, current.as_bytes())
            })
            .and_then(|()| {
                recovery.mark_copy_pending(
                    &recovery.mirrored_copy(&dir, request.file_path),
                )
            })
            .map_err(|error| {
                StepError::Failed(format!("{}: {error}", dir.display()))
            })?;
        Ok(Some(dir))
    }

    fn captured_by_last_sync(&self, request: &PullRequest<'_>) -> bool {
        let path = request.file_path;
        match (
            self.files.tracking.file_tracking(path),
            request.unchanged_since_sync,
        ) {
            (
                FileTracking::Tracked,
                UnchangedSinceSync::RecipeOne | UnchangedSinceSync::Legacy,
            ) => self.status.is_dirty(path) == Dirtiness::Clean,
            (FileTracking::Untracked, UnchangedSinceSync::RecipeOne) => true,
            _ => false,
        }
    }

    fn pulled_fingerprints(
        &self,
        request: &PullRequest<'_>,
    ) -> Result<DualRecipeFingerprints, StepError<String>> {
        self.digester()
            .digests(request.content, Some(request.projected_body))
            .map_err(|error| StepError::Failed(error.to_string()))
    }

    fn write_item(
        &self,
        request: &PullRequest<'_>,
    ) -> Result<(), PublishError<StoreError>> {
        self.files
            .work_items
            .write(request.file_path, request.content.as_bytes())
            .map_err(|error| item_write_error(request.id, error))
    }

    fn set_pulled_baseline(
        &self,
        request: &PullRequest<'_>,
        fingerprints: DualRecipeFingerprints,
    ) -> Result<(), StepError<String>> {
        self.set_baseline(request.id, &request.remote_updated, fingerprints)
    }

    fn set_baseline(
        &self,
        id: &str,
        remote_updated: &RemoteTimestamp,
        fingerprints: DualRecipeFingerprints,
    ) -> Result<(), StepError<String>> {
        self.baseline
            .set(
                id,
                Entry::recorded(
                    SyncStamps {
                        remote_updated_at: remote_updated.clone(),
                        local_synced_at: self.run_start_epoch,
                    },
                    fingerprints,
                ),
            )
            .map_err(|error| StepError::Failed(error.to_string()))
    }

    fn lock_item(
        &self,
        request: &PullRequest<'_>,
    ) -> Result<HeldLock, StepError<StoreError>> {
        self.files
            .file_locks
            .acquire(&LockName::ForFile(request.file_path.to_path_buf()))
            .map_err(StepError::Failed)
    }

    /// Authors a discovered remote issue as a new local file and records its
    /// baseline entry, so the next run classifies it as synced. Returns the
    /// allocated local id.
    ///
    /// The `show` fetches the projected body the local file and the baseline
    /// both need — `Discovery` carries only ids and stamps. The exclusive
    /// authoring write lives behind [`LocalAuthor`], so an id collision
    /// surfaces as an error here rather than a silent clobber.
    pub fn create_from_remote(
        &mut self,
        external_id: &ExternalId,
        author: &dyn LocalAuthor,
    ) -> Attempted<String> {
        let mut attempt = ItemAttempt::begin(external_id.as_str());
        let result =
            self.create_from_remote_steps(external_id, author, &mut attempt);
        attempt.finish(result)
    }

    fn create_from_remote_steps(
        &self,
        external_id: &ExternalId,
        author: &dyn LocalAuthor,
        attempt: &mut ItemAttempt,
    ) -> Result<String, ApplyError> {
        let issue = attempt.observe_tracker(SyncOperation::Show, || {
            self.tracker.show(external_id)
        })?;
        let body = BodyComparison::new(self.tracker, self.parser)
            .local_body(&issue.body)
            .map_err(|cause| ApplyError::RemoteUnrenderable {
                item_id: external_id.as_str().to_owned(),
                operation: SyncOperation::Create,
                cause,
            })?;
        let title = remote_projection::split_projected(&issue.body)
            .title
            .to_owned();
        let authored = attempt.land_local(
            SyncOperation::AuthorLocal,
            Some(Landing::Author),
            || {
                author
                    .author_from_remote(&DiscoveredIssue {
                        external_id: external_id.clone(),
                        title,
                        body,
                        issue: issue.clone(),
                    })
                    .map_err(|error| {
                        PublishError::Step(StepError::Failed(error))
                    })
            },
        )?;
        attempt.known_as(&authored.id);
        let local_content = attempt
            .observe_local(SyncOperation::ReadLocal, || {
                read_file(&authored.path)
            })?;
        self.record(attempt, &local_content, Some(&issue))?;
        Ok(authored.id)
    }

    /// Creates a remote issue for an unsynced local draft, links the returned
    /// id back into the file, and records the baseline entry.
    ///
    /// The durable [`crate::sync::pending_push`] marker is written **before**
    /// the non-idempotent `create`, so a crash in the window between a
    /// successful remote create and the local link is recoverable: the next
    /// run reads the marker and reuses the id rather than creating a duplicate.
    pub fn create_from_local(
        &mut self,
        request: &CreateFromLocalRequest<'_>,
    ) -> Attempted<CreatedLocally> {
        let mut attempt = ItemAttempt::begin(request.item_id);
        attempt.keeping_marker(request.marker_path);
        let result = self.create_from_local_steps(request, &mut attempt);
        attempt.finish(result)
    }

    fn create_from_local_steps(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
    ) -> Result<CreatedLocally, ApplyError> {
        let digest = pending_push::request_digest(
            request.title,
            request.body,
            request.kind,
        );

        let marker = self.read_marker(request);
        if let Some(order) = request.order {
            return self.settle_order(request, attempt, &marker, digest, order);
        }
        let is_carried = |key: &ExternalId| (request.carrier_of)(key).is_some();
        match push_precondition(&marker.state(), &digest, &is_carried) {
            PushPrecondition::Refuse(reason) => {
                let trusted = marker.trusted();
                let carrier = match trusted {
                    Some(PendingPush::Created { external_id, .. }) => {
                        (request.carrier_of)(external_id)
                    }
                    _ => None,
                };
                let state = marker.distrust().map_or_else(
                    || refused_marker_state(reason, trusted, carrier),
                    str::to_owned,
                );
                Err(ApplyError::PendingPushRefused {
                    item_id: request.item_id.to_owned(),
                    reason,
                    detail: format!(
                        "{state} (marker {})",
                        request.marker_path.display()
                    ),
                })
            }
            PushPrecondition::ReuseId(external_id) => {
                if let Some(blocking) = request.corpus_blocked_by {
                    return Err(ApplyError::CorpusIncomplete {
                        subject: request.item_id.to_owned(),
                        deferred: IncompleteCorpusDeferral::ReuseId(
                            external_id,
                        ),
                        blocking,
                    });
                }
                self.link_and_baseline(request, attempt, &external_id)
                    .map(CreatedLocally::from)
            }
            PushPrecondition::Proceed => self
                .create_marking_progress(request, attempt, digest)
                .map(CreatedLocally::from),
        }
    }

    /// The item's pending-push marker, distrusted when a person could have
    /// committed it or when it records a key no tracker issues.
    fn read_marker(
        &self,
        request: &CreateFromLocalRequest<'_>,
    ) -> MarkerReading {
        let raw = std::fs::read_to_string(request.marker_path).ok();
        let marker = match pending_push::read(raw.as_deref()) {
            Err(_) => {
                return MarkerReading::Unreadable;
            }
            Ok(None) => return MarkerReading::Absent,
            Ok(Some(marker)) => marker,
        };
        let created = match &marker {
            PendingPush::Created { external_id, .. } => Some(external_id),
            PendingPush::Attempted { .. } => None,
        };
        let issued_key = created
            .filter(|key| TrackerKey::parse(key.as_str()).is_some())
            .cloned();
        if self.files.tracking.file_tracking(request.marker_path)
            != FileTracking::Untracked
        {
            return MarkerReading::Distrusted {
                cause: "marker is tracked by version control".to_owned(),
                created: issued_key,
            };
        }
        if created.is_some() && issued_key.is_none() {
            return MarkerReading::Distrusted {
                cause: format!(
                    "marker records a key that is not a {} key",
                    request.tracker_name
                ),
                created: None,
            };
        }
        MarkerReading::Trusted(marker)
    }

    /// Carries out a person's settlement of a refused pending push. Adopt
    /// never falls through to a create, and neither order links or creates
    /// an issue the tracker has not confirmed.
    fn settle_order(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        reading: &MarkerReading,
        digest: String,
        order: &PendingPushOrder,
    ) -> Result<CreatedLocally, ApplyError> {
        let marker = &reading.state();
        let refused = |refusal| ApplyError::OrderRefused {
            item_id: request.item_id.to_owned(),
            order: order.clone(),
            refusal: Box::new(refusal),
        };
        if let (PendingPushOrder::Adopt(key), Some(blocking)) =
            (order, request.corpus_blocked_by)
        {
            return Err(ApplyError::CorpusIncomplete {
                subject: request.item_id.to_owned(),
                deferred: IncompleteCorpusDeferral::Adopt(key.into()),
                blocking,
            });
        }
        let is_carried = |key: &ExternalId| (request.carrier_of)(key).is_some();
        let carrier_of = |key: &ExternalId| {
            (request.carrier_of)(key).unwrap_or_else(|| {
                unreachable!("the order was refused because {key} is carried")
            })
        };
        let has_marker = !matches!(marker, MarkerState::Absent);
        let decision = match (reading, order) {
            (
                MarkerReading::Distrusted {
                    created: Some(recorded),
                    ..
                },
                PendingPushOrder::Create,
            ) => OrderPrecondition::CreateUnlessIssueExists(recorded.clone()),
            _ => order_precondition(marker, &digest, order, &is_carried),
        };
        match decision {
            OrderPrecondition::AlreadyWritten => {
                let carrier = match marker {
                    MarkerState::Present(PendingPush::Created {
                        external_id,
                        ..
                    }) => carrier_of(external_id),
                    _ => unreachable!(
                        "only a created marker's issue is already written"
                    ),
                };
                Err(refused(OrderRefusal::AlreadyWritten { carrier }))
            }
            OrderPrecondition::KeyCarried => {
                let key = adopted_key(order);
                let carrier = carrier_of(&key);
                Err(refused(OrderRefusal::KeyCarried { key, carrier }))
            }
            OrderPrecondition::Adopt => self.adopt(
                request,
                attempt,
                &adopted_key(order),
                has_marker,
                refused,
            ),
            OrderPrecondition::AdoptIfMarkerKeyMovedHere(recorded) => {
                let key = adopted_key(order);
                match attempt.observe_tracker(SyncOperation::Adopt, || {
                    self.tracker.locate(&recorded)
                })? {
                    Located::Found(issue) if same_key(&issue.key, &key) => {
                        self.adopt(request, attempt, &key, has_marker, refused)
                    }
                    _ => Err(refused(OrderRefusal::MarkerRecords {
                        id: request.item_id.to_owned(),
                        key: recorded,
                    })),
                }
            }
            OrderPrecondition::Create => self
                .create_marking_progress(request, attempt, digest)
                .map(CreatedLocally::from),
            OrderPrecondition::CreateUnmarked => self
                .create_marking_progress(request, attempt, digest)
                .map(|leftover_marker| CreatedLocally {
                    notice: Some(CreateNotice::NothingToSettle),
                    ..CreatedLocally::from(leftover_marker)
                }),
            OrderPrecondition::CreateUnlessIssueExists(recorded) => {
                match attempt.observe_tracker(SyncOperation::Create, || {
                    self.tracker.locate(&recorded)
                })? {
                    Located::NotFound => self
                        .create_marking_progress(request, attempt, digest)
                        .map(CreatedLocally::from),
                    Located::Found(issue) => {
                        Err(refused(OrderRefusal::MarkerRecords {
                            id: request.item_id.to_owned(),
                            key: issue.key,
                        }))
                    }
                }
            }
        }
    }

    /// Links `key` once the tracker confirms it exists under that key, and
    /// records the baseline only when the issue already says what the item
    /// does: a mistyped key naming an unrelated issue then shows as an
    /// obviously wrong conflict before anything is pushed.
    fn adopt(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        key: &ExternalId,
        has_marker: bool,
        refused: impl Fn(OrderRefusal) -> ApplyError,
    ) -> Result<CreatedLocally, ApplyError> {
        let issue = match attempt
            .observe_tracker(SyncOperation::Adopt, || {
                self.tracker.locate(key)
            })? {
            Located::NotFound => {
                return Err(refused(OrderRefusal::IssueNotFound {
                    key: key.clone(),
                }));
            }
            Located::Found(issue) => issue,
        };
        if !same_key(&issue.key, key) {
            return Err(refused(
                TrackerKey::parse(issue.key.as_str()).map_or_else(
                    || OrderRefusal::MovedToUnknownKey { key: key.clone() },
                    |to| OrderRefusal::IssueMoved {
                        id: request.item_id.to_owned(),
                        key: key.clone(),
                        to,
                    },
                ),
            ));
        }
        attempt.land_local(
            SyncOperation::LinkExternalId,
            Some(Landing::Link),
            || {
                request
                    .author
                    .link_external_id(request.file_path, key)
                    .map_err(|error| {
                        PublishError::Step(StepError::Failed(error))
                    })
            },
        )?;
        let differs = !self.says_what_the_item_does(request, &issue);
        if !differs {
            let local_content = attempt
                .observe_local(SyncOperation::ReadLocal, || {
                    read_file(request.file_path)
                })?;
            self.record(attempt, &local_content, Some(&issue))?;
        }
        Ok(CreatedLocally {
            leftover_marker: if has_marker {
                self.clear_marker(request.marker_path).err()
            } else {
                None
            },
            notice: differs
                .then(|| CreateNotice::DifferingAdoption(key.clone())),
            adopted: Some(Adoption {
                id: request.item_id.to_owned(),
                key: key.clone(),
                title: issue_title(&issue),
            }),
        })
    }

    fn says_what_the_item_does(
        &self,
        request: &CreateFromLocalRequest<'_>,
        issue: &RemoteIssue,
    ) -> bool {
        issue_title(issue).trim() == request.title.trim()
            && BodyComparison::new(self.tracker, self.parser)
                .equivalence(request.body, &issue.body)
                == BodyEquivalence::Equivalent
    }

    fn create_marking_progress(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        digest: String,
    ) -> Result<Option<LeftoverMarker>, ApplyError> {
        let fingerprint = RequestFingerprint {
            title: request.title.to_owned(),
            digest,
            attempted_at: request.attempted_at,
            failure: None,
        };
        self.write_marker(
            attempt,
            request.marker_path,
            &PendingPush::Attempted {
                request: fingerprint.clone(),
            },
        )?;

        match attempt.land_created(|| {
            self.tracker
                .create(request.title, request.body, request.kind)
        }) {
            Ok(external_id) => self.record_created(
                request,
                attempt,
                &fingerprint,
                &external_id,
            ),
            Err(TrackerError::Terminal { detail }) => self
                .record_terminal_failure(request, attempt, fingerprint, detail),
            Err(source) => Err(self.abandon_attempt(request, attempt, source)),
        }
    }

    fn record_created(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        fingerprint: &RequestFingerprint,
        external_id: &ExternalId,
    ) -> Result<Option<LeftoverMarker>, ApplyError> {
        self.write_marker(
            attempt,
            request.marker_path,
            &PendingPush::Created {
                request: fingerprint.clone(),
                external_id: external_id.clone(),
            },
        )?;
        self.link_and_baseline(request, attempt, external_id)
    }

    /// A marker left behind here would refuse the next run's create as a
    /// prior attempt of unknown outcome, so failing to clear it is the
    /// item's failure.
    fn abandon_attempt(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &ItemAttempt,
        source: TrackerError,
    ) -> ApplyError {
        match self.clear_marker(request.marker_path) {
            Ok(()) => attempt.tracker_failure(SyncOperation::Create, source),
            Err(leftover) => attempt.io_failure(
                SyncOperation::MarkerWrite,
                format!(
                    "could not clear marker {}: {}",
                    leftover.path.display(),
                    leftover.reason
                ),
            ),
        }
    }

    fn clear_marker(&self, path: &Path) -> Result<(), LeftoverMarker> {
        self.files.integration_state.remove(path).map_err(|error| {
            LeftoverMarker {
                path: path.to_path_buf(),
                reason: error.to_string(),
            }
        })
    }

    fn record_terminal_failure(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        fingerprint: RequestFingerprint,
        detail: String,
    ) -> Result<Option<LeftoverMarker>, ApplyError> {
        self.write_marker(
            attempt,
            request.marker_path,
            &PendingPush::Attempted {
                request: RequestFingerprint {
                    failure: Some(detail.clone()),
                    ..fingerprint
                },
            },
        )?;
        Err(attempt.tracker_failure(
            SyncOperation::Create,
            TrackerError::Terminal { detail },
        ))
    }

    fn write_marker(
        &self,
        attempt: &mut ItemAttempt,
        path: &Path,
        marker: &PendingPush,
    ) -> Result<(), ApplyError> {
        attempt.land_local(SyncOperation::MarkerWrite, None, || {
            self.files
                .integration_state
                .write(path, pending_push::render(marker).as_bytes())
                .map_err(|error| marker_write_error(path, &error))
        })
    }

    fn link_and_baseline(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
        external_id: &ExternalId,
    ) -> Result<Option<LeftoverMarker>, ApplyError> {
        attempt.land_local(
            SyncOperation::LinkExternalId,
            Some(Landing::Link),
            || {
                request
                    .author
                    .link_external_id(request.file_path, external_id)
                    .map_err(|error| {
                        PublishError::Step(StepError::Failed(error))
                    })
            },
        )?;

        let remote = self.tracker.show(external_id).ok();
        let local_content = attempt
            .observe_local(SyncOperation::ReadLocal, || {
                read_file(request.file_path)
            })?;
        self.record(attempt, &local_content, remote.as_ref())?;
        Ok(self.clear_marker(request.marker_path).err())
    }

    /// Records `request`'s fingerprints, writing no file and calling no
    /// tracker, once the local file still digests to what was classified.
    ///
    /// The result fails with [`ApplyError::LocalChangedSinceClassified`]
    /// when the file moved since the run read it, so an edit is never
    /// absorbed into the baseline; with [`ApplyError::Io`] when it cannot
    /// be read or the baseline cannot be written.
    pub fn record_baseline(
        &mut self,
        request: &RecordRequest<'_>,
    ) -> Attempted<()> {
        let attempt = ItemAttempt::begin(request.id);
        let result = self.record_baseline_steps(request, &attempt);
        attempt.finish(result)
    }

    fn record_baseline_steps(
        &self,
        request: &RecordRequest<'_>,
        attempt: &ItemAttempt,
    ) -> Result<(), ApplyError> {
        let content = attempt
            .observe_local(SyncOperation::ReadLocal, || {
                read_file(request.file_path)
            })?;
        let current = attempt
            .observe_local(SyncOperation::HashLocal, || {
                digest::local(&content).map_err(StepError::Failed)
            })?;
        if request.fingerprints.legacy.local.as_ref() != Some(&current) {
            return Err(ApplyError::LocalChangedSinceClassified {
                item_id: request.id.to_owned(),
            });
        }
        attempt.observe_local(SyncOperation::BaselineSet, || {
            self.set_baseline(
                request.id,
                &request.remote_updated,
                request.fingerprints.clone(),
            )
        })
    }

    fn digester(&self) -> BaselineDigester<'ctx> {
        BaselineDigester::new(self.tracker, self.parser)
    }

    /// Records the attempt's item from what was just written locally and
    /// read back remotely; an unread remote records no remote fingerprints,
    /// so the next run reads that side as changed.
    fn record(
        &self,
        attempt: &ItemAttempt,
        local_content: &str,
        remote: Option<&RemoteIssue>,
    ) -> Result<(), ApplyError> {
        let fingerprints =
            attempt.observe_local(SyncOperation::HashLocal, || {
                self.digester()
                    .digests(
                        local_content,
                        remote.map(|issue| issue.body.as_str()),
                    )
                    .map_err(|error| StepError::Failed(error.to_string()))
            })?;
        let remote_updated = remote
            .map_or(RemoteTimestamp::NotRead, |issue| issue.updated.clone());
        attempt.observe_local(SyncOperation::BaselineSet, || {
            self.set_baseline(attempt.item_id(), &remote_updated, fingerprints)
        })
    }
}

/// The title line of an issue's projected body.
pub fn issue_title(issue: &RemoteIssue) -> String {
    remote_projection::split_projected(&issue.body)
        .title
        .to_owned()
}

fn adopted_key(order: &PendingPushOrder) -> ExternalId {
    match order {
        PendingPushOrder::Adopt(key) => key.into(),
        PendingPushOrder::Create => {
            unreachable!("only an adopt order names a key to adopt")
        }
    }
}

fn same_key(left: &ExternalId, right: &ExternalId) -> bool {
    canonical_external_key(left) == canonical_external_key(right)
}

fn read_local(
    request: &PullRequest<'_>,
) -> Result<String, StepError<std::io::Error>> {
    read_file(request.file_path)
}

fn read_file(path: &Path) -> Result<String, StepError<std::io::Error>> {
    std::fs::read_to_string(path).map_err(StepError::Failed)
}

fn ensure_unchanged_since_classified(
    request: &PullRequest<'_>,
    current: &str,
) -> Result<(), ApplyError> {
    if current == request.classified_content || current == request.content {
        Ok(())
    } else {
        Err(ApplyError::LocalChangedSinceClassified {
            item_id: request.id.to_owned(),
        })
    }
}
