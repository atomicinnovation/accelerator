//! The per-item apply sequence: push, pull and finalise, over the frozen
//! `tracker::RemoteTracker` port and a `BaselineStore`.

use std::path::Path;

use corpus::body::BodyParser;
use corpus::store::AtomicWrite;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker::Unrenderable;
use work::sync::push_precondition;
use work::sync::DualRecipeFingerprints;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;

use crate::sync::baseline::Entry;
use crate::sync::baseline::SyncStamps;
use crate::sync::baseline_digester::BaselineDigester;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::body_comparison::BodyComparison;
use crate::sync::create::DiscoveredIssue;
use crate::sync::create::LocalAuthor;
use crate::sync::digest;
use crate::sync::pending_push;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    Terminal,
    Unconfigured,
    Rejected,
    Unrenderable,
    PullResidue,
}

#[derive(Debug)]
pub enum ApplyError {
    Tracker {
        item_id: String,
        operation: &'static str,
        source: TrackerError,
    },
    Io {
        item_id: String,
        operation: &'static str,
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
    },
    LocalChangedSinceClassified {
        item_id: String,
    },
}

impl ApplyError {
    /// A local read or write failure is terminal: whether a write landed is
    /// unknown, so a blind retry is unsafe.
    #[must_use]
    pub const fn class(&self) -> FailureClass {
        match self {
            Self::Tracker { source, .. } => match source {
                TrackerError::Retryable { .. } => FailureClass::Retryable,
                TrackerError::Terminal { .. } => FailureClass::Terminal,
                TrackerError::Unconfigured { .. } => FailureClass::Unconfigured,
                TrackerError::Rejected { .. } => FailureClass::Rejected,
            },
            Self::Io { .. } => FailureClass::Terminal,
            Self::RemoteBodyUnavailable { .. }
            | Self::LocalChangedSinceClassified { .. } => {
                FailureClass::Retryable
            }
            Self::RemoteUnrenderable { .. } => FailureClass::Unrenderable,
            Self::LocalPullResidue { .. } => FailureClass::PullResidue,
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
            } => write!(formatter, "{item_id}: {operation} failed: {detail}"),
            Self::RemoteBodyUnavailable { item_id } => write!(
                formatter,
                "{item_id}: pull failed: the remote body was not fetched"
            ),
            Self::RemoteUnrenderable { item_id, cause } => {
                write!(formatter, "{item_id}: {cause}")
            }
            Self::LocalPullResidue { item_id } => write!(
                formatter,
                "{item_id}: the local file still holds the raw tracker \
                 document an earlier pull wrote, so nothing was sent; replace \
                 it with Markdown (`jira show <key> --render-adf` prints a \
                 starting point) and sync, or resolve with --resolve \
                 {item_id}=remote to discard local changes"
            ),
            Self::LocalChangedSinceClassified { item_id } => write!(
                formatter,
                "{item_id}: the local file changed while the run was \
                 recording its baseline, so nothing was recorded; re-run the \
                 sync"
            ),
        }
    }
}

impl std::error::Error for ApplyError {}

pub struct PushRequest<'a> {
    pub id: &'a str,
    pub external_id: &'a ExternalId,
    pub title: &'a str,
    pub body: &'a str,
    pub file_path: &'a Path,
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
    pub content: &'a str,
    /// Un-normalised, and hashed here rather than by the caller, so the
    /// baseline's `remote_hash` always comes from the projection actually
    /// written.
    pub projected_body: &'a str,
    pub remote_updated: RemoteTimestamp,
}

fn io_error(
    id: &str,
    operation: &'static str,
    detail: impl std::fmt::Display,
) -> ApplyError {
    ApplyError::Io {
        item_id: id.to_owned(),
        operation,
        detail: detail.to_string(),
    }
}

/// Applies one planned action, over the tracker port and a baseline store.
///
/// The store's borrow (`'store`) stays separate from the collaborators'
/// (`'ctx`) because a single lifetime is invariant: `&'a mut
/// BaselineStore<'a>` would borrow the store for the whole of the applier's
/// own lifetime, leaving the baseline unreadable afterwards.
pub struct ItemApplier<'ctx, 'store> {
    tracker: &'ctx dyn RemoteTracker,
    parser: &'ctx dyn BodyParser,
    writer: &'ctx dyn AtomicWrite,
    baseline: &'store mut BaselineStore<'ctx>,
    run_start_epoch: u64,
}

impl<'ctx, 'store> ItemApplier<'ctx, 'store> {
    pub fn new(
        tracker: &'ctx dyn RemoteTracker,
        parser: &'ctx dyn BodyParser,
        writer: &'ctx dyn AtomicWrite,
        baseline: &'store mut BaselineStore<'ctx>,
        run_start_epoch: u64,
    ) -> Self {
        Self {
            tracker,
            parser,
            writer,
            baseline,
            run_start_epoch,
        }
    }

    /// The baseline entry is written last, so a failed `update` leaves it
    /// unset and the next run reclassifies from scratch. A failed post-push
    /// `show` still writes the entry, with both remote fields empty and
    /// [`RemoteTimestamp::NotRead`] — the only place that variant is
    /// written.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] naming `request.id` and the operation attempted.
    pub fn push(
        &mut self,
        request: &PushRequest<'_>,
    ) -> Result<(), ApplyError> {
        self.tracker
            .update(request.external_id, request.title, request.body)
            .map_err(|source| ApplyError::Tracker {
                item_id: request.id.to_owned(),
                operation: "update",
                source,
            })?;

        let remote = self.tracker.show(request.external_id).ok();
        let local_content = std::fs::read_to_string(request.file_path)
            .map_err(|error| io_error(request.id, "read-local", error))?;
        self.record(request.id, &local_content, remote.as_ref())
    }

    /// Both baseline hashes come from what was actually written. Deriving
    /// either from pre-pull content self-corrupts the baseline into a
    /// phantom `locally-modified` on the next run.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] naming `request.id` and the operation attempted.
    pub fn pull(
        &mut self,
        request: &PullRequest<'_>,
    ) -> Result<(), ApplyError> {
        self.writer
            .write(request.file_path, request.content.as_bytes())
            .map_err(|error| io_error(request.id, "write-local", error))?;

        let fingerprints = self
            .digester()
            .digests(request.content, Some(request.projected_body))
            .map_err(|error| io_error(request.id, "hash-local", error))?;
        self.baseline
            .set(
                request.id,
                Entry::recorded(
                    SyncStamps {
                        remote_updated_at: request.remote_updated.clone(),
                        local_synced_at: self.run_start_epoch,
                    },
                    fingerprints,
                ),
            )
            .map_err(|error| io_error(request.id, "baseline-set", error))
    }

    /// Authors a discovered remote issue as a new local file and records its
    /// baseline entry, so the next run classifies it as synced. Returns the
    /// allocated local id.
    ///
    /// The `show` fetches the projected body the local file and the baseline
    /// both need — `Discovery` carries only ids and stamps. The exclusive
    /// authoring write lives behind [`LocalAuthor`], so an id collision
    /// surfaces as an error here rather than a silent clobber.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] when the `show`, the authoring write, or the baseline
    /// write fails.
    pub fn create_from_remote(
        &mut self,
        external_id: &ExternalId,
        author: &dyn LocalAuthor,
    ) -> Result<String, ApplyError> {
        let issue = self.tracker.show(external_id).map_err(|source| {
            ApplyError::Tracker {
                item_id: external_id.as_str().to_owned(),
                operation: "show",
                source,
            }
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
        let authored = author
            .author_from_remote(&DiscoveredIssue {
                external_id: external_id.clone(),
                title,
                body,
                issue: issue.clone(),
            })
            .map_err(|error| {
                io_error(external_id.as_str(), "author-local", error)
            })?;

        let local_content = std::fs::read_to_string(&authored.path)
            .map_err(|error| io_error(&authored.id, "read-local", error))?;
        self.record(&authored.id, &local_content, Some(&issue))?;
        Ok(authored.id)
    }

    /// Creates a remote issue for an unsynced local draft, links the returned
    /// id back into the file, and records the baseline entry.
    ///
    /// The durable [`crate::sync::pending_push`] marker is written **before**
    /// the non-idempotent `create`, so a crash in the window between a
    /// successful remote create and the local link is recoverable: the next
    /// run reads the marker and reuses the id rather than creating a duplicate.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] when the marker refuses the attempt, the `create` fails,
    /// or a filesystem/baseline write fails.
    pub fn create_from_local(
        &mut self,
        request: &CreateFromLocalRequest<'_>,
    ) -> Result<(), ApplyError> {
        let digest = pending_push::request_digest(
            request.title,
            request.body,
            request.kind,
        );

        match Self::push_decision(request, &digest) {
            PushPrecondition::Refuse(reason) => Err(io_error(
                request.item_id,
                "create",
                refusal_detail(reason, request.marker_path),
            )),
            PushPrecondition::ReuseId(external_id) => {
                self.link_and_baseline(request, &external_id)
            }
            PushPrecondition::Proceed => {
                self.create_marking_progress(request, digest)
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
        digest: String,
    ) -> Result<(), ApplyError> {
        let fingerprint = RequestFingerprint {
            title: request.title.to_owned(),
            digest,
            attempted_at: request.attempted_at,
            failure: None,
        };
        self.write_marker(
            request.marker_path,
            &PendingPush::Attempted {
                request: fingerprint.clone(),
            },
        )?;

        match self
            .tracker
            .create(request.title, request.body, request.kind)
        {
            Ok(external_id) => {
                self.record_created(request, &fingerprint, &external_id)
            }
            Err(
                source @ (TrackerError::Retryable { .. }
                | TrackerError::Unconfigured { .. }
                | TrackerError::Rejected { .. }),
            ) => Self::abandon_attempt(request, source),
            Err(TrackerError::Terminal { detail }) => {
                self.record_terminal_failure(request, fingerprint, detail)
            }
        }
    }

    fn record_created(
        &self,
        request: &CreateFromLocalRequest<'_>,
        fingerprint: &RequestFingerprint,
        external_id: &ExternalId,
    ) -> Result<(), ApplyError> {
        self.write_marker(
            request.marker_path,
            &PendingPush::Created {
                request: fingerprint.clone(),
                external_id: external_id.clone(),
            },
        )?;
        self.link_and_baseline(request, external_id)
    }

    fn abandon_attempt(
        request: &CreateFromLocalRequest<'_>,
        source: TrackerError,
    ) -> Result<(), ApplyError> {
        std::fs::remove_file(request.marker_path).ok();
        Err(ApplyError::Tracker {
            item_id: request.item_id.to_owned(),
            operation: "create",
            source,
        })
    }

    fn record_terminal_failure(
        &self,
        request: &CreateFromLocalRequest<'_>,
        fingerprint: RequestFingerprint,
        detail: String,
    ) -> Result<(), ApplyError> {
        self.write_marker(
            request.marker_path,
            &PendingPush::Attempted {
                request: RequestFingerprint {
                    failure: Some(detail.clone()),
                    ..fingerprint
                },
            },
        )?;
        Err(ApplyError::Tracker {
            item_id: request.item_id.to_owned(),
            operation: "create",
            source: TrackerError::Terminal { detail },
        })
    }

    fn write_marker(
        &self,
        path: &Path,
        marker: &PendingPush,
    ) -> Result<(), ApplyError> {
        self.writer
            .write(path, pending_push::render(marker).as_bytes())
            .map_err(|error| io_error("<marker>", "marker-write", error))
    }

    fn link_and_baseline(
        &self,
        request: &CreateFromLocalRequest<'_>,
        external_id: &ExternalId,
    ) -> Result<(), ApplyError> {
        request
            .author
            .link_external_id(request.file_path, external_id)
            .map_err(|error| {
                io_error(request.item_id, "link-external-id", error)
            })?;

        let remote = self.tracker.show(external_id).ok();
        let local_content = std::fs::read_to_string(request.file_path)
            .map_err(|error| io_error(request.item_id, "read-local", error))?;
        self.record(request.item_id, &local_content, remote.as_ref())?;
        std::fs::remove_file(request.marker_path).ok();
        Ok(())
    }

    /// Records `request`'s fingerprints, writing no file and calling no
    /// tracker, once the local file still digests to what was classified.
    ///
    /// # Errors
    ///
    /// [`ApplyError::LocalChangedSinceClassified`] when the file moved
    /// since it was classified, so an edit is never absorbed into the
    /// baseline; [`ApplyError::Io`] when it cannot be read or the baseline
    /// cannot be written.
    pub fn record_baseline(
        &self,
        request: &RecordRequest<'_>,
    ) -> Result<(), ApplyError> {
        let content = std::fs::read_to_string(request.file_path)
            .map_err(|error| io_error(request.id, "read-local", error))?;
        let current = digest::local(&content)
            .map_err(|error| io_error(request.id, "hash-local", error))?;
        if request.fingerprints.legacy.local.as_ref() != Some(&current) {
            return Err(ApplyError::LocalChangedSinceClassified {
                item_id: request.id.to_owned(),
            });
        }
        self.baseline
            .set(
                request.id,
                Entry::recorded(
                    SyncStamps {
                        remote_updated_at: request.remote_updated.clone(),
                        local_synced_at: self.run_start_epoch,
                    },
                    request.fingerprints.clone(),
                ),
            )
            .map_err(|error| io_error(request.id, "baseline-set", error))
    }

    fn digester(&self) -> BaselineDigester<'ctx> {
        BaselineDigester::new(self.tracker, self.parser)
    }

    /// Records `id`'s entry from what was just written locally and read
    /// back remotely; an unread remote records no remote fingerprints, so
    /// the next run reads that side as changed.
    fn record(
        &self,
        id: &str,
        local_content: &str,
        remote: Option<&RemoteIssue>,
    ) -> Result<(), ApplyError> {
        let fingerprints = self
            .digester()
            .digests(local_content, remote.map(|issue| issue.body.as_str()))
            .map_err(|error| io_error(id, "hash-local", error))?;
        let stamps = SyncStamps {
            remote_updated_at: remote
                .map_or(RemoteTimestamp::NotRead, |issue| {
                    issue.updated.clone()
                }),
            local_synced_at: self.run_start_epoch,
        };
        self.baseline
            .set(id, Entry::recorded(stamps, fingerprints))
            .map_err(|error| io_error(id, "baseline-set", error))
    }

    /// Advances the baseline's global timestamp, blanking nothing: the run
    /// blanks its own unreconciled items through its `BaselineStore` before
    /// calling this.
    ///
    /// # Errors
    ///
    /// [`ApplyError`] when the underlying store operation fails.
    pub fn finalise(&mut self, run_start_epoch: u64) -> Result<(), ApplyError> {
        self.baseline
            .finalise_run(&[], &[], run_start_epoch, true)
            .map_err(|error| io_error("<run>", "finalise", error))
    }
}
