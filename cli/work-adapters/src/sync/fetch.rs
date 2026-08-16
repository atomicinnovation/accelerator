//! The fetch shell: the imperative half of gathering remote facts.
//!
//! Performs `fetch_all`/`show` and local dirtiness probes, then hands the
//! gathered facts to `work::sync::plan`, which owns every rule. This
//! module owns no rule of its own beyond the two-tier read
//! (bulk-then-`show`) and the retrieval-strategy dispatch.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use tracker::ExternalId;
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

pub struct GatheredFacts {
    pub per_id: BTreeMap<String, (GatheredRemote, Dirtiness)>,
    /// A `fetch_all` pre-flight failure. Every present id is marked
    /// `Indeterminate` and this is carried through so the run can report
    /// it — discarding it turns a misconfigured token into a whole-corpus
    /// "nothing to do".
    pub read_failure: Option<TrackerError>,
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
        baseline_timestamp: u64,
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
                    baseline_timestamp,
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

/// Gathers the facts `work::sync::plan` needs, for every item.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn gather(
    items: &[LocalItem],
    baseline: &Baseline,
    tracker: &dyn RemoteTracker,
    status: &dyn WorkingCopyStatus,
    strategy: RetrievalStrategy,
) -> GatheredFacts {
    let mut per_id = BTreeMap::new();
    let mut read_failure = None;
    let present = present_ids(items);

    match strategy {
        RetrievalStrategy::PerItem => {
            for (id, external_id) in &present {
                let remote = match tracker.show(external_id) {
                    Ok(issue) => GatheredRemote {
                        presence: RemotePresence::Present,
                        remote_updated: issue.updated,
                        body: Some(issue.body),
                    },
                    Err(_) => GatheredRemote {
                        presence: RemotePresence::Indeterminate,
                        remote_updated: RemoteTimestamp::NotReported,
                        body: None,
                    },
                };
                per_id.insert((*id).to_owned(), remote);
            }
        }
        RetrievalStrategy::Bulk => {
            let ids: Vec<ExternalId> =
                present.iter().map(|(_, id)| (*id).clone()).collect();
            match tracker.fetch_all(&ids) {
                Err(error) => {
                    read_failure = Some(error);
                    for (id, _) in &present {
                        per_id.insert(
                            (*id).to_owned(),
                            GatheredRemote {
                                presence: RemotePresence::Indeterminate,
                                remote_updated: RemoteTimestamp::NotReported,
                                body: None,
                            },
                        );
                    }
                }
                Ok(outcome) => {
                    for (id, external_id) in &present {
                        if let Some((_, stamp)) = outcome
                            .found
                            .iter()
                            .find(|(found_id, _)| found_id == *external_id)
                        {
                            let baseline_updated = baseline
                                .get(id)
                                .map(|entry| &entry.remote_updated_at);
                            let unchanged =
                                baseline_updated.is_some_and(|known| {
                                    stamp.proves_unchanged_since(known)
                                });
                            let body = if unchanged {
                                None
                            } else {
                                tracker
                                    .show(external_id)
                                    .ok()
                                    .map(|issue| issue.body)
                            };
                            per_id.insert(
                                (*id).to_owned(),
                                GatheredRemote {
                                    presence: RemotePresence::Present,
                                    remote_updated: stamp.clone(),
                                    body,
                                },
                            );
                        } else if outcome.absent.contains(external_id) {
                            per_id.insert(
                                (*id).to_owned(),
                                GatheredRemote {
                                    presence: RemotePresence::Absent,
                                    remote_updated:
                                        RemoteTimestamp::NotReported,
                                    body: None,
                                },
                            );
                        } else {
                            per_id.insert(
                                (*id).to_owned(),
                                GatheredRemote {
                                    presence: RemotePresence::Indeterminate,
                                    remote_updated:
                                        RemoteTimestamp::NotReported,
                                    body: None,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let mut per_id_with_dirty = BTreeMap::new();
    for item in items {
        let remote = per_id.remove(&item.id).unwrap_or_else(placeholder_remote);
        let dirty = status.is_dirty(&item.path);
        per_id_with_dirty.insert(item.id.clone(), (remote, dirty));
    }

    GatheredFacts {
        per_id: per_id_with_dirty,
        read_failure,
    }
}
