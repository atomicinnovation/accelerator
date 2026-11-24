//! What a push's read-back says, and what that means for the entry's
//! rewrite mark and the notice the push reports.

use std::path::Path;

use tracker::ExternalId;

use crate::sync::body_comparison::BodyComparison;
use crate::sync::push_fidelity::PushFidelity;
use crate::sync::push_fidelity::RewriteNotice;
use crate::sync::rewrite_mark::MarkChange;

/// Where the body a read-back is judged against came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOrigin {
    /// This run pushed the body to an issue that already existed.
    Pushed,
    /// This run created the issue with the body.
    Created,
    /// An earlier run created the issue, so this run sent nothing to judge.
    CreatedEarlier,
}

/// A push's read-back judged for where its body came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushVerdict {
    origin: PushOrigin,
    fidelity: PushFidelity,
}

impl PushVerdict {
    /// The verdict on `read_back` after `sent` reached the tracker from
    /// `origin`.
    #[must_use]
    pub fn judged(
        origin: PushOrigin,
        comparison: &BodyComparison<'_>,
        sent: &str,
        read_back: Option<&str>,
    ) -> Self {
        let fidelity = match origin {
            PushOrigin::Pushed | PushOrigin::Created => {
                comparison.push_fidelity(sent, read_back)
            }
            PushOrigin::CreatedEarlier => PushFidelity::Unjudged,
        };
        Self { origin, fidelity }
    }

    /// The verdict when nothing could be read back.
    #[must_use]
    pub const fn unread(origin: PushOrigin) -> Self {
        Self {
            origin,
            fidelity: PushFidelity::Unjudged,
        }
    }

    /// How recording this push treats the entry's rewrite mark.
    #[must_use]
    pub const fn mark_change(&self) -> MarkChange<'_> {
        match self.origin {
            PushOrigin::Pushed => MarkChange::Judged(&self.fidelity),
            PushOrigin::Created | PushOrigin::CreatedEarlier => {
                MarkChange::Fresh(&self.fidelity)
            }
        }
    }

    #[must_use]
    pub fn into_fidelity(self) -> PushFidelity {
        self.fidelity
    }

    /// The notice this push reports for the item at `path` known remotely
    /// as `external_id`: one only when the tracker rewrote it.
    #[must_use]
    pub fn into_notice(
        self,
        path: &Path,
        external_id: &ExternalId,
    ) -> Option<RewriteNotice> {
        RewriteNotice::on_push(self.fidelity, path, external_id)
    }
}
