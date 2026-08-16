//! The per-item apply sequence: push, pull and finalise, over the frozen
//! `tracker::RemoteTracker` port and a `BaselineStore`.

use std::path::Path;

use corpus::store::AtomicWrite;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;

use crate::sync::baseline::Entry;
use crate::sync::baseline_store::BaselineStore;
use crate::sync::digest;

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
