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
use work::sync::BaselineFingerprints;
use work::sync::Dirtiness;
use work::sync::LegacyHashes;
use work::sync::PlanInput;
use work::sync::RemoteFacts;
use work::sync::RemotePresence;

use crate::sync::baseline::Baseline;
use crate::sync::baseline::Entry;
use crate::sync::digest;
use crate::sync::item_snapshot::ItemSnapshot;

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
    /// Borrowed planner inputs, paired with each item's snapshot.
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
        snapshots: &'a [ItemSnapshot<'a>],
        baseline: &'a Baseline,
    ) -> Vec<PlanInput<'a>> {
        items
            .iter()
            .zip(snapshots.iter())
            .map(|(item, snapshot)| {
                let (remote, dirty) = self
                    .per_id
                    .get(&item.id)
                    .expect("gather populates every item");
                let baseline_entry = baseline.get(&item.id);
                let stamps = baseline_entry.and_then(Entry::stamps);
                let watermark = stamps.map_or_else(
                    || baseline.timestamp(),
                    |stamps| stamps.local_synced_at,
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
                        remote_updated_at: stamps
                            .map_or(&RemoteTimestamp::NotRead, |stamps| {
                                &stamps.remote_updated_at
                            }),
                        fingerprints: baseline_entry
                            .and_then(Entry::fingerprints)
                            .unwrap_or_else(|| {
                                BaselineFingerprints::Legacy(
                                    LegacyHashes::default(),
                                )
                            }),
                    },
                    baseline_timestamp: watermark,
                    digests: snapshot,
                    comparison: snapshot,
                }
            })
            .collect()
    }
}

fn present_ids(items: &[LocalItem]) -> Vec<(&str, &ExternalId, &Path)> {
    items
        .iter()
        .filter_map(|item| {
            item.external_id
                .as_ref()
                .map(|id| (item.id.as_str(), id, item.path.as_path()))
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
            for (id, external_id, _) in &present {
                if recipe_unsupported(baseline, id) {
                    gathering
                        .record(id, unaccounted(RemotePresence::Indeterminate));
                    continue;
                }
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
                present.iter().map(|(_, id, _)| (*id).clone()).collect();
            match tracker.fetch_all(&ids) {
                Err(error) => {
                    read_failure = Some(error);
                    for (id, _, _) in &present {
                        gathering.record(
                            id,
                            unaccounted(RemotePresence::Indeterminate),
                        );
                    }
                }
                Ok(outcome) => {
                    keyed_read = outcome.completeness;
                    for (id, external_id, path) in &present {
                        gather_from_bulk(
                            &mut gathering,
                            id,
                            external_id,
                            path,
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

/// Whether `id`'s entry was recorded under a digest recipe this binary does
/// not know, so nothing about the item can be compared, read or written.
fn recipe_unsupported(baseline: &Baseline, id: &str) -> bool {
    matches!(baseline.get(id), Some(Entry::Unsupported { .. }))
}

/// Whether the entry lets the stamp alone stand for the remote body: only a
/// recipe-1 entry whose stamp proves the remote unchanged and whose local
/// file still digests as recorded. A legacy entry always needs the body,
/// so the run can rewrite it, or find the residue an earlier pull left.
fn remote_read_unneeded(
    path: &Path,
    entry: &Entry,
    stamp: &RemoteTimestamp,
) -> bool {
    matches!(entry, Entry::Split { .. })
        && entry.stamps().is_some_and(|stamps| {
            stamp.proves_unchanged_since(&stamps.remote_updated_at)
        })
        && local_side_unmoved(path, entry)
}

/// Whether the local file still hashes to the entry's recorded digest. A
/// send is planned only for a moved local side, and a send must see the
/// fetched remote, so any doubt here forces the read.
fn local_side_unmoved(path: &Path, entry: &Entry) -> bool {
    let Some(recorded) =
        entry.legacy().and_then(|legacy| legacy.local.as_ref())
    else {
        return false;
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| digest::local(&content).ok())
        .is_some_and(|local_hash| &local_hash == recorded)
}

fn gather_from_bulk(
    gathering: &mut Gathering,
    id: &str,
    external_id: &ExternalId,
    path: &Path,
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
    if recipe_unsupported(baseline, id) {
        gathering.record(
            id,
            GatheredRemote {
                presence: RemotePresence::Indeterminate,
                remote_updated: stamp.clone(),
                body: None,
            },
        );
        return;
    }
    let unchanged = baseline
        .get(id)
        .is_some_and(|entry| remote_read_unneeded(path, entry, stamp));
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
                presence: RemotePresence::Indeterminate,
                remote_updated: stamp.clone(),
                body: None,
            },
        ),
    }
}
