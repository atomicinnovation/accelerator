//! The fetch shell: the imperative half of gathering remote facts.
//!
//! Performs `fetch_all`/`show` and local dirtiness probes, then hands the
//! gathered facts to `work::sync::plan`, which owns every rule. This
//! module owns no rule of its own beyond the two-tier read
//! (bulk-then-`show`) and the retrieval-strategy dispatch.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use tracker::Completeness;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::Located;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work::sync::BaselineEntry;
use work::sync::Dirtiness;
use work::sync::PlanInput;
use work::sync::RemoteFacts;
use work::sync::RemotePresence;

use crate::sync::baseline::Baseline;
use crate::sync::digest::LazyItemDigests;

#[derive(Clone, Debug)]
pub struct LocalItem {
    pub id: String,
    pub path: PathBuf,
    pub external_id: Option<ExternalId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalStrategy {
    Bulk,
    PerItem,
}

/// A local working-copy status probe.
///
/// Injected so the fetch shell never shells `jj`/`git` itself. Whether a
/// probe covers the whole tree once or one path at a time is a property of
/// the implementation, not of this interface.
pub trait WorkingCopyStatus {
    fn is_dirty(&self, path: &Path) -> Dirtiness;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatheredRemote {
    pub presence: RemotePresence,
    pub remote_updated: RemoteTimestamp,
    pub body: Option<String>,
}

/// A read that answered under a key other than the one the item stores, or
/// answered that no issue has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityObservation {
    Moved { old: ExternalId, new: ExternalId },
    NotFound { key: ExternalId },
}

pub struct GatheredFacts {
    pub per_id: BTreeMap<String, (GatheredRemote, Dirtiness)>,
    /// Keyed by item id. A moved item's `per_id` fact describes the issue
    /// under its new key.
    pub identity: BTreeMap<String, IdentityObservation>,
    /// A `fetch_all` pre-flight failure. Every present id is marked
    /// `Indeterminate` and this is carried through so the run can report
    /// it — discarding it turns a misconfigured token into a whole-corpus
    /// "nothing to do".
    pub read_failure: Option<TrackerError>,
    /// Whether the bulk keyed read saw everything in scope, and why not when it
    /// did not. A [`Completeness::CapHit`] is the fail-loud signal the run
    /// aborts on before any write; a [`Completeness::Transient`] surfaces
    /// softly and the run proceeds with the affected items indeterminate. The
    /// per-item (`show`) strategy and a pre-flight `fetch_all` failure both
    /// leave this [`Completeness::Complete`].
    pub keyed_read: Completeness,
}

impl GatheredFacts {
    /// Borrowed planner inputs, paired with each item's digest port.
    ///
    /// Built on demand rather than stored, since `PlanInput`'s borrowed
    /// views cannot be held inside the same struct as the owned facts they
    /// borrow from.
    ///
    /// # Panics
    ///
    /// If `items` names an id `gather` did not populate — a caller bug,
    /// since `gather` always populates every item it was given.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn plan_inputs<'a>(
        &'a self,
        items: &'a [LocalItem],
        digests: &'a [LazyItemDigests<'a>],
        baseline: &'a Baseline,
    ) -> Vec<PlanInput<'a>> {
        items
            .iter()
            .zip(digests.iter())
            .map(|(item, item_digests)| {
                let (remote, dirty) = self
                    .per_id
                    .get(&item.id)
                    .expect("gather populates every item");
                let baseline_entry = baseline.get(&item.id);
                let watermark = baseline_entry.map_or_else(
                    || baseline.timestamp(),
                    |entry| entry.local_synced_at,
                );
                PlanInput {
                    id: item.id.clone(),
                    external_id: item.external_id.as_ref(),
                    facts: RemoteFacts {
                        presence: remote.presence,
                        remote_updated: &remote.remote_updated,
                    },
                    dirty: *dirty,
                    baseline: BaselineEntry {
                        remote_updated_at: baseline_entry
                            .map_or(&RemoteTimestamp::NotRead, |entry| {
                                &entry.remote_updated_at
                            }),
                        remote_hash: baseline_entry.and_then(|entry| {
                            (!entry.remote_hash.is_empty())
                                .then_some(entry.remote_hash.as_str())
                        }),
                        local_hash: baseline_entry.and_then(|entry| {
                            (!entry.local_hash.is_empty())
                                .then_some(entry.local_hash.as_str())
                        }),
                    },
                    baseline_timestamp: watermark,
                    digests: item_digests,
                }
            })
            .collect()
    }
}

fn present_ids(items: &[LocalItem]) -> Vec<(&str, &ExternalId)> {
    items
        .iter()
        .filter_map(|item| {
            item.external_id.as_ref().map(|id| (item.id.as_str(), id))
        })
        .collect()
}

const fn placeholder_remote() -> GatheredRemote {
    GatheredRemote {
        presence: RemotePresence::Present,
        remote_updated: RemoteTimestamp::NotReported,
        body: None,
    }
}

const fn unaccounted(presence: RemotePresence) -> GatheredRemote {
    GatheredRemote {
        presence,
        remote_updated: RemoteTimestamp::NotReported,
        body: None,
    }
}

fn is_move(requested: &ExternalId, answered: &ExternalId) -> bool {
    !requested.as_str().eq_ignore_ascii_case(answered.as_str())
}

/// Accumulates one item's gathered facts, noting a key change wherever a
/// read answers under a key other than the one requested.
struct Gathering {
    per_id: BTreeMap<String, GatheredRemote>,
    identity: BTreeMap<String, IdentityObservation>,
}

impl Gathering {
    fn record(&mut self, id: &str, remote: GatheredRemote) {
        self.per_id.insert(id.to_owned(), remote);
    }

    fn record_issue(
        &mut self,
        id: &str,
        requested: &ExternalId,
        issue: RemoteIssue,
    ) {
        if is_move(requested, &issue.key) {
            self.identity.insert(
                id.to_owned(),
                IdentityObservation::Moved {
                    old: requested.clone(),
                    new: issue.key,
                },
            );
        }
        self.record(
            id,
            GatheredRemote {
                presence: RemotePresence::Present,
                remote_updated: issue.updated,
                body: Some(issue.body),
            },
        );
    }

    /// Asks the tracker whether an issue its bulk read could not account
    /// for has moved or is gone; a failed answer keeps what the bulk read
    /// said.
    fn locate(
        &mut self,
        id: &str,
        requested: &ExternalId,
        tracker: &dyn RemoteTracker,
        otherwise: RemotePresence,
    ) {
        match tracker.locate(requested) {
            Ok(Located::Found(issue)) => {
                self.record_issue(id, requested, issue);
            }
            Ok(Located::NotFound) => {
                self.identity.insert(
                    id.to_owned(),
                    IdentityObservation::NotFound {
                        key: requested.clone(),
                    },
                );
                self.record(id, unaccounted(RemotePresence::Absent));
            }
            Err(_) => self.record(id, unaccounted(otherwise)),
        }
    }
}

/// Gathers the facts `work::sync::plan` needs, for every item.
#[must_use]
pub fn gather(
    items: &[LocalItem],
    baseline: &Baseline,
    tracker: &dyn RemoteTracker,
    status: &dyn WorkingCopyStatus,
    strategy: RetrievalStrategy,
) -> GatheredFacts {
    let mut gathering = Gathering {
        per_id: BTreeMap::new(),
        identity: BTreeMap::new(),
    };
    let mut read_failure = None;
    let mut keyed_read = Completeness::Complete;
    let present = present_ids(items);

    match strategy {
        RetrievalStrategy::PerItem => {
            for (id, external_id) in &present {
                match tracker.show(external_id) {
                    Ok(issue) => gathering.record_issue(id, external_id, issue),
                    Err(_) => gathering.locate(
                        id,
                        external_id,
                        tracker,
                        RemotePresence::Indeterminate,
                    ),
                }
            }
        }
        RetrievalStrategy::Bulk => {
            let ids: Vec<ExternalId> =
                present.iter().map(|(_, id)| (*id).clone()).collect();
            match tracker.fetch_all(&ids) {
                Err(error) => {
                    read_failure = Some(error);
                    for (id, _) in &present {
                        gathering.record(
                            id,
                            unaccounted(RemotePresence::Indeterminate),
                        );
                    }
                }
                Ok(outcome) => {
                    keyed_read = outcome.completeness;
                    for (id, external_id) in &present {
                        gather_from_bulk(
                            &mut gathering,
                            id,
                            external_id,
                            &outcome,
                            baseline,
                            tracker,
                        );
                    }
                }
            }
        }
    }

    let mut per_id_with_dirty = BTreeMap::new();
    for item in items {
        let remote = gathering
            .per_id
            .remove(&item.id)
            .unwrap_or_else(placeholder_remote);
        let dirty = status.is_dirty(&item.path);
        per_id_with_dirty.insert(item.id.clone(), (remote, dirty));
    }

    GatheredFacts {
        per_id: per_id_with_dirty,
        identity: gathering.identity,
        read_failure,
        keyed_read,
    }
}

fn gather_from_bulk(
    gathering: &mut Gathering,
    id: &str,
    external_id: &ExternalId,
    outcome: &FetchOutcome,
    baseline: &Baseline,
    tracker: &dyn RemoteTracker,
) {
    let found = outcome
        .found
        .iter()
        .find(|(found_id, _)| found_id == external_id);
    let Some((_, stamp)) = found else {
        let otherwise = if outcome.absent.contains(external_id) {
            RemotePresence::Absent
        } else {
            RemotePresence::Indeterminate
        };
        gathering.locate(id, external_id, tracker, otherwise);
        return;
    };
    let unchanged = baseline.get(id).is_some_and(|entry| {
        stamp.proves_unchanged_since(&entry.remote_updated_at)
    });
    if unchanged {
        gathering.record(
            id,
            GatheredRemote {
                presence: RemotePresence::Present,
                remote_updated: stamp.clone(),
                body: None,
            },
        );
        return;
    }
    match tracker.show(external_id) {
        Ok(issue) => gathering.record_issue(
            id,
            external_id,
            RemoteIssue {
                updated: stamp.clone(),
                ..issue
            },
        ),
        Err(_) => gathering.record(
            id,
            GatheredRemote {
                presence: RemotePresence::Present,
                remote_updated: stamp.clone(),
                body: None,
            },
        ),
    }
}
