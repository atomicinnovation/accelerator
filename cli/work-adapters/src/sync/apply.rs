//! The per-item apply sequence: push, pull and finalise, over the frozen
//! `tracker::RemoteTracker` port and a `BaselineStore`.

use std::path::Path;
use std::path::PathBuf;

use corpus::body::BodyParser;
use corpus::lock::HeldLock;
use corpus::lock::LockName;
use corpus::store::StoreError;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker::Unrenderable;
use vcs::tracking::FileTracking;
use work::sync::push_precondition;
use work::sync::Dirtiness;
use work::sync::DualRecipeFingerprints;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;

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
use crate::sync::create::DiscoveredIssue;
use crate::sync::create::LocalAuthor;
use crate::sync::digest;
use crate::sync::fetch::WorkingCopyStatus;
use crate::sync::files::SyncFiles;
use crate::sync::operation::SyncOperation;
use crate::sync::pending_push;
use crate::sync::recovery_label::RecoveryLabel;

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
    },
    RemoteUnrenderable {
        item_id: String,
        cause: Unrenderable,
    },
    LocalPullResidue {
        item_id: String,
        external_id: ExternalId,
    },
    LocalUnparseable {
        item_id: String,
        detail: String,
    },
    LocalWriteRefused {
        item_id: String,
        detail: String,
    },
    LocalChangedSinceClassified {
        item_id: String,
    },
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
            | Self::LocalChangedSinceClassified { .. } => {
                FailureClass::Retryable
            }
            Self::Io {
                progress: ItemProgress::MayHaveLanded,
                ..
            } => FailureClass::Terminal,
            Self::RemoteUnrenderable { .. } => FailureClass::Unrenderable,
            Self::LocalPullResidue { .. } => FailureClass::PullResidue,
            Self::LocalUnparseable { .. } => FailureClass::Unparseable,
            Self::LocalWriteRefused { .. } => FailureClass::WriteRefused,
        }
    }
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tracker {
                item_id,
                operation,
                source,
            } => write!(formatter, "{item_id}: {operation} failed: {source}"),
            Self::Io {
                item_id,
                operation,
                detail,
                ..
            } => write!(formatter, "{item_id}: {operation} failed: {detail}"),
            Self::RemoteBodyUnavailable { item_id } => write!(
                formatter,
                "{item_id}: the remote description was not fetched this run, \
                 so nothing was sent or written; re-run the sync"
            ),
            Self::RemoteUnrenderable { item_id, cause } => {
                write!(formatter, "{item_id}: {cause}")
            }
            Self::LocalPullResidue {
                item_id,
                external_id,
            } => write!(
                formatter,
                "{item_id}: the local file still holds the raw tracker \
                 document an earlier pull wrote, so nothing was sent; replace \
                 it with Markdown (`jira show {external_id} --render-adf` \
                 prints a starting point) and sync, or resolve with --resolve \
                 {item_id}=remote to discard local changes"
            ),
            Self::LocalUnparseable { item_id, detail } => write!(
                formatter,
                "{item_id}: the local file's frontmatter does not parse \
                 ({detail}), so nothing was sent or written; fix the file and \
                 re-run the sync"
            ),
            Self::LocalWriteRefused { item_id, detail } => write!(
                formatter,
                "{item_id}: the local file was not written ({detail}), so it \
                 is as it was; the same write is refused until that changes"
            ),
            Self::LocalChangedSinceClassified { item_id } => write!(
                formatter,
                "{item_id}: the local file changed after the run read it, so \
                 nothing was written or recorded; re-run the sync"
            ),
        }
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

/// The inputs `create_from_local` needs beyond the applier's own ports.
///
/// `marker_path` is precomputed by the run so the applier stays ignorant of
/// the integrations layout, and `corpus_carries` reports whether any local
/// item already carries a candidate `external_id` — the guard that stops a
/// recovered `Created` marker binding two files to one remote issue.
pub struct CreateFromLocalRequest<'a> {
    pub item_id: &'a str,
    pub file_path: &'a Path,
    pub title: &'a str,
    pub body: &'a str,
    pub kind: &'a str,
    pub marker_path: &'a Path,
    pub author: &'a dyn LocalAuthor,
    pub corpus_carries: &'a dyn Fn(&ExternalId) -> bool,
    pub attempted_at: u64,
}

fn refusal_detail(reason: RefusalReason, marker_path: &Path) -> String {
    let marker = marker_path.display();
    match reason {
        RefusalReason::MarkerUnreadable => format!(
            "pending-push marker at {marker} could not be parsed; a previous \
             create may have partially applied — inspect or remove it"
        ),
        RefusalReason::PriorAttemptUnknownOutcome => format!(
            "a previous create attempt recorded at {marker} has an unknown \
             outcome — a remote issue may already exist; inspect it, then \
             remove the marker to retry"
        ),
        RefusalReason::FingerprintMismatch => format!(
            "the pending-push marker at {marker} was recorded for a different \
             request; remove it to force a new create"
        ),
        RefusalReason::AlreadyWritten => format!(
            "the pending-push marker at {marker} names an external_id already \
             carried by a work item on disk; remove it if this is a genuine \
             duplicate create"
        ),
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
    ) -> Attempted<Option<LeftoverMarker>> {
        let mut attempt = ItemAttempt::begin(request.item_id);
        attempt.keeping_marker(request.marker_path);
        let result = self.create_from_local_steps(request, &mut attempt);
        attempt.finish(result)
    }

    fn create_from_local_steps(
        &self,
        request: &CreateFromLocalRequest<'_>,
        attempt: &mut ItemAttempt,
    ) -> Result<Option<LeftoverMarker>, ApplyError> {
        let digest = pending_push::request_digest(
            request.title,
            request.body,
            request.kind,
        );

        match Self::push_decision(request, &digest) {
            PushPrecondition::Refuse(reason) => Err(ApplyError::Io {
                item_id: request.item_id.to_owned(),
                operation: SyncOperation::Create,
                progress: ItemProgress::MayHaveLanded,
                detail: refusal_detail(reason, request.marker_path),
            }),
            PushPrecondition::ReuseId(external_id) => {
                self.link_and_baseline(request, attempt, &external_id)
            }
            PushPrecondition::Proceed => {
                self.create_marking_progress(request, attempt, digest)
            }
        }
    }

    fn push_decision(
        request: &CreateFromLocalRequest<'_>,
        digest: &str,
    ) -> PushPrecondition {
        let marker_content = std::fs::read_to_string(request.marker_path).ok();
        let parsed = pending_push::read(marker_content.as_deref());
        let marker_state = match &parsed {
            Err(_) => MarkerState::Unreadable,
            Ok(None) => MarkerState::Absent,
            Ok(Some(marker)) => MarkerState::Present(marker),
        };
        push_precondition(&marker_state, digest, request.corpus_carries)
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
