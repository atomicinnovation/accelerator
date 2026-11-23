//! Each queued service's gate, queue and contention log, built together so
//! one call names the service once and the three can never disagree.

use std::path::PathBuf;
use std::rc::Rc;

use research::sources::fetch::Clock;
use research::sources::queue::Nonce;
use research::sources::queue::Provenance;
use research::sources::service::CrossrefPool;
use research::sources::service::Service;

use crate::contention::FileContentionLog;
use crate::diagnostics::Diagnostics;
use crate::pacing::FilePacingGate;
use crate::queue::FileServiceQueue;
use crate::queue::ForeignSource;
use crate::scratch::ScratchDir;

/// What every service's adapters share.
pub struct AdapterContext {
    pub research: ScratchDir,
    pub clock: Rc<dyn Clock>,
    pub diagnostics: Rc<dyn Diagnostics>,
    pub nonces: Rc<dyn Fn() -> Nonce>,
}

/// Where the release before per-service state kept arXiv's queue and lock.
pub struct PreviousRelease {
    queue: ScratchDir,
    lock: PathBuf,
}

impl PreviousRelease {
    pub fn arxiv_layout(research: &ScratchDir) -> Self {
        Self {
            queue: research.nested("arxiv-queue"),
            lock: research.path("arxiv.lock"),
        }
    }
}

pub struct ServiceAdapters {
    pub gate: FilePacingGate,
    pub queue: FileServiceQueue,
    pub contention: FileContentionLog,
}

impl ServiceAdapters {
    /// While a previous release may still run, arXiv's gate also holds its
    /// lock, so the two releases keep one arXiv connection at a time, and
    /// its tickets are refused rather than joined afresh.
    pub fn for_arxiv(
        context: &AdapterContext,
        previous_release: Option<PreviousRelease>,
    ) -> Self {
        Self::of(Service::Arxiv, context, previous_release)
    }

    pub fn for_crossref(pool: CrossrefPool, context: &AdapterContext) -> Self {
        Self::of(Service::Crossref(pool), context, None)
    }

    fn of(
        service: Service,
        context: &AdapterContext,
        previous_release: Option<PreviousRelease>,
    ) -> Self {
        let mut foreign = other_family_queues(service, &context.research);
        let mut companions = Vec::new();
        if let Some(previous) = previous_release {
            foreign.push(ForeignSource {
                directory: previous.queue,
                provenance: Provenance::IssuedBeforeUpgrade,
            });
            companions.push(previous.lock);
        }
        let nonces = context.nonces.clone();
        Self {
            gate: FilePacingGate::new(
                service,
                &context.research,
                context.clock.clone(),
                context.diagnostics.clone(),
                companions,
            ),
            queue: FileServiceQueue::new(
                service,
                &context.research,
                context.clock.clone(),
                context.diagnostics.clone(),
                Box::new(move || nonces()),
                foreign,
            ),
            contention: FileContentionLog::new(
                service,
                &context.research,
                context.clock.clone(),
                context.diagnostics.clone(),
            ),
        }
    }
}

/// The queue of every service issuing another family's tickets. Pools of
/// one family are not foreign to each other.
fn other_family_queues(
    service: Service,
    research: &ScratchDir,
) -> Vec<ForeignSource> {
    Service::ALL
        .into_iter()
        .filter(|other| other.ticket_family() != service.ticket_family())
        .map(|other| ForeignSource {
            directory: FileServiceQueue::directory_of(other, research),
            provenance: Provenance::IssuedElsewhere(other.ticket_family()),
        })
        .collect()
}
