//! The per-item apply sequence: push, pull and finalise, over the frozen
//! `tracker::RemoteTracker` port and a `BaselineStore`.

use std::path::Path;

use corpus::store::AtomicWrite;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work::sync::push_precondition;
use work::sync::MarkerState;
use work::sync::PendingPush;
use work::sync::PushPrecondition;
use work::sync::RefusalReason;
use work::sync::RequestFingerprint;

use crate::sync::baseline::Entry;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::create::DiscoveredIssue;
use crate::sync::create::LocalAuthor;
use crate::sync::digest;
use crate::sync::pending_push;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    Terminal,
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
}

impl ApplyError {
    /// `None` for a store-originated failure, which carries no class the
    /// report can render.
    #[must_use]
    pub const fn class(&self) -> Option<FailureClass> {
        match self {
            Self::Tracker { source, .. } => Some(match source {
                TrackerError::Retryable { .. } => FailureClass::Retryable,
                TrackerError::Terminal { .. } => FailureClass::Terminal,
            }),
            Self::Io { .. } => None,
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
    writer: &'ctx dyn AtomicWrite,
    baseline: &'store mut BaselineStore<'ctx>,
}

impl<'ctx, 'store> ItemApplier<'ctx, 'store> {
    pub fn new(
        tracker: &'ctx dyn RemoteTracker,
        writer: &'ctx dyn AtomicWrite,
        baseline: &'store mut BaselineStore<'ctx>,
    ) -> Self {
        Self {
            tracker,
            writer,
            baseline,
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

        let (remote_updated, remote_hash) =
            match self.tracker.show(request.external_id) {
                Ok(issue) => (issue.updated, digest::remote_body(&issue.body)),
                Err(_) => (RemoteTimestamp::NotRead, String::new()),
            };

        let local_content = std::fs::read_to_string(request.file_path)
            .map_err(|error| io_error(request.id, "read-local", error))?;
        let local_hash = digest::local(&local_content)
            .map_err(|error| io_error(request.id, "hash-local", error))?;

        self.baseline
            .set(
                request.id,
                Entry {
                    remote_updated_at: remote_updated,
                    remote_hash,
                    local_hash,
                },
            )
            .map_err(|error| io_error(request.id, "baseline-set", error))
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

        let local_hash = digest::local(request.content)
            .map_err(|error| io_error(request.id, "hash-local", error))?;
        let remote_hash = digest::remote_body(request.projected_body);

        self.baseline
            .set(
                request.id,
                Entry {
                    remote_updated_at: request.remote_updated.clone(),
                    remote_hash,
                    local_hash,
                },
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
        let authored = author
            .author_from_remote(&DiscoveredIssue {
                external_id: external_id.clone(),
                issue: issue.clone(),
            })
            .map_err(|error| {
                io_error(external_id.as_str(), "author-local", error)
            })?;

        let local_content = std::fs::read_to_string(&authored.path)
            .map_err(|error| io_error(&authored.id, "read-local", error))?;
        let local_hash = digest::local(&local_content)
            .map_err(|error| io_error(&authored.id, "hash-local", error))?;
        let remote_hash = digest::remote_body(&issue.body);

        self.baseline
            .set(
                &authored.id,
                Entry {
                    remote_updated_at: issue.updated,
                    remote_hash,
                    local_hash,
                },
            )
            .map_err(|error| io_error(&authored.id, "baseline-set", error))?;
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
        let marker_content = std::fs::read_to_string(request.marker_path).ok();
        let parsed = pending_push::read(marker_content.as_deref());
        let digest = pending_push::request_digest(
            request.title,
            request.body,
            request.kind,
        );
        let marker_state = match &parsed {
            Err(_) => MarkerState::Unreadable,
            Ok(None) => MarkerState::Absent,
            Ok(Some(marker)) => MarkerState::Present(marker),
        };

        match push_precondition(&marker_state, &digest, request.corpus_carries)
        {
            PushPrecondition::Refuse(reason) => Err(io_error(
                request.item_id,
                "create",
                refusal_detail(reason, request.marker_path),
            )),
            PushPrecondition::ReuseId(external_id) => {
                self.link_and_baseline(request, &external_id)
            }
            PushPrecondition::Proceed => {
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
                match self.tracker.create(
                    request.title,
                    request.body,
                    request.kind,
                ) {
                    Ok(external_id) => {
                        self.write_marker(
                            request.marker_path,
                            &PendingPush::Created {
                                request: fingerprint,
                                external_id: external_id.clone(),
                            },
                        )?;
                        self.link_and_baseline(request, &external_id)
                    }
                    Err(source @ TrackerError::Retryable { .. }) => {
                        std::fs::remove_file(request.marker_path).ok();
                        Err(ApplyError::Tracker {
                            item_id: request.item_id.to_owned(),
                            operation: "create",
                            source,
                        })
                    }
                    Err(TrackerError::Terminal { detail }) => {
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
                }
            }
        }
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
        &mut self,
        request: &CreateFromLocalRequest<'_>,
        external_id: &ExternalId,
    ) -> Result<(), ApplyError> {
        request
            .author
            .link_external_id(request.file_path, external_id)
            .map_err(|error| {
                io_error(request.item_id, "link-external-id", error)
            })?;

        let (remote_updated, remote_hash) = match self.tracker.show(external_id)
        {
            Ok(issue) => (issue.updated, digest::remote_body(&issue.body)),
            Err(_) => (RemoteTimestamp::NotRead, String::new()),
        };
        let local_content = std::fs::read_to_string(request.file_path)
            .map_err(|error| io_error(request.item_id, "read-local", error))?;
        let local_hash = digest::local(&local_content)
            .map_err(|error| io_error(request.item_id, "hash-local", error))?;

        self.baseline
            .set(
                request.item_id,
                Entry {
                    remote_updated_at: remote_updated,
                    remote_hash,
                    local_hash,
                },
            )
            .map_err(|error| {
                io_error(request.item_id, "baseline-set", error)
            })?;
        std::fs::remove_file(request.marker_path).ok();
        Ok(())
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
            .finalise_run(&[], run_start_epoch)
            .map_err(|error| io_error("<run>", "finalise", error))
    }
}
