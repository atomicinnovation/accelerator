//! The port as an external consumer sees it: a fake implementation that
//! stops compiling if any signature moves, exercised through a trait object
//! because the sync engine's composition root holds one.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;

struct FixedTracker {
    known: Vec<(ExternalId, RemoteIssue)>,
    unprovable: Vec<ExternalId>,
    lossy: Vec<ExternalId>,
}

impl FixedTracker {
    const fn holding(known: Vec<(ExternalId, RemoteIssue)>) -> Self {
        Self {
            known,
            unprovable: Vec::new(),
            lossy: Vec::new(),
        }
    }

    const fn truncating(
        known: Vec<(ExternalId, RemoteIssue)>,
        unprovable: Vec<ExternalId>,
    ) -> Self {
        Self {
            known,
            unprovable,
            lossy: Vec::new(),
        }
    }

    /// Ids whose write is acknowledged by nothing — the response is lost, so
    /// the fake cannot say whether the mutation applied.
    const fn losing(
        known: Vec<(ExternalId, RemoteIssue)>,
        lossy: Vec<ExternalId>,
    ) -> Self {
        Self {
            known,
            unprovable: Vec::new(),
            lossy,
        }
    }

    fn issue(&self, id: &ExternalId) -> Option<&RemoteIssue> {
        self.known
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, issue)| issue)
    }
}

impl RemoteTracker for FixedTracker {
    fn create(
        &self,
        _title: &str,
        _body: &str,
        _kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        Ok(ExternalId::new(format!("ENG-{}", self.known.len() + 1)))
    }

    fn update(
        &self,
        id: &ExternalId,
        _title: &str,
        _body: &str,
    ) -> Result<(), TrackerError> {
        if self.lossy.contains(id) {
            return Err(TrackerError::Terminal {
                detail: format!(
                    "jira: update {id} failed, response lost after send"
                ),
            });
        }
        if self.issue(id).is_some() {
            return Ok(());
        }
        Err(TrackerError::Retryable {
            detail: format!(
                "jira: update {id} rejected, HTTP 404 no such issue"
            ),
        })
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.issue(id)
            .cloned()
            .ok_or_else(|| TrackerError::Retryable {
                detail: format!("fake: show {id} failed, connection refused"),
            })
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        let mut outcome = FetchOutcome {
            found: Vec::new(),
            absent: Vec::new(),
            indeterminate: Vec::new(),
        };
        let mut requested: Vec<&ExternalId> = ids.iter().collect();
        requested.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        requested.dedup();
        for id in requested {
            match self.issue(id) {
                Some(issue) => {
                    outcome.found.push((id.clone(), issue.updated.clone()));
                }
                None if self.unprovable.contains(id) => {
                    outcome.indeterminate.push(id.clone());
                }
                None => outcome.absent.push(id.clone()),
            }
        }
        Ok(outcome)
    }
}

const JIRA_STAMP: &str = "2026-07-09T08:00:00.000+0000";

fn issue(stamp: &str, body: &str) -> RemoteIssue {
    RemoteIssue {
        updated: RemoteTimestamp::new(stamp.to_owned()),
        body: body.to_owned(),
    }
}

fn known() -> Vec<(ExternalId, RemoteIssue)> {
    vec![(
        ExternalId::new("ENG-1".to_owned()),
        issue(JIRA_STAMP, "Pushed title\nPushed description\n"),
    )]
}

#[test]
fn all_four_operations_are_reachable_through_a_trait_object() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(known()));

    let created = tracker
        .create("Add remote tracker port", "Body text\n", "story")
        .expect("the fake always creates");
    assert_eq!(created.as_str(), "ENG-2");

    let id = ExternalId::new("ENG-1".to_owned());
    assert_eq!(tracker.update(&id, "Title", "Body text\n"), Ok(()));
    assert_eq!(
        tracker.show(&id).expect("the fake holds ENG-1").body,
        "Pushed title\nPushed description\n"
    );
    let outcome = tracker
        .fetch_all(std::slice::from_ref(&id))
        .expect("the fake fetches");
    assert_eq!(
        outcome.found,
        vec![(id, RemoteTimestamp::new(JIRA_STAMP.to_owned()))]
    );
}

/// A compile-time echo of the surface pin, on the stable lane.
///
/// `public-api:check` runs only on the nightly architecture job, so a nightly
/// break or a rustdoc-JSON skew takes the freeze offline while every stable
/// check stays green. Exhaustive destructuring costs nothing and means an added
/// or removed public field reddens `cargo nextest` too.
#[test]
fn every_public_field_is_accounted_for() {
    let tracker = FixedTracker::holding(known());
    let id = ExternalId::new("ENG-1".to_owned());

    let RemoteIssue { updated, body } =
        tracker.show(&id).expect("the fake holds ENG-1");
    let FetchOutcome {
        found,
        absent,
        indeterminate,
    } = tracker.fetch_all(&[id]).expect("the fake fetches");

    assert_eq!(updated.as_str(), JIRA_STAMP);
    assert!(!body.is_empty());
    assert_eq!(found.len(), 1);
    assert!(absent.is_empty() && indeterminate.is_empty());
}

#[test]
fn a_failed_read_is_retryable_because_it_mutated_nothing() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(Vec::new()));
    let missing = ExternalId::new("ENG-404".to_owned());

    assert_eq!(
        tracker.show(&missing),
        Err(TrackerError::Retryable {
            detail: "fake: show ENG-404 failed, connection refused".to_owned()
        })
    );
}

#[test]
fn a_rejected_write_is_retryable_because_nothing_was_modified() {
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::holding(Vec::new()));
    let missing = ExternalId::new("ENG-404".to_owned());

    assert_eq!(
        tracker.update(&missing, "Title", "Body"),
        Err(TrackerError::Retryable {
            detail: "jira: update ENG-404 rejected, HTTP 404 no such issue"
                .to_owned()
        })
    );
}

#[test]
fn a_write_whose_response_was_lost_is_terminal() {
    let id = ExternalId::new("ENG-1".to_owned());
    let tracker: Box<dyn RemoteTracker> =
        Box::new(FixedTracker::losing(known(), vec![id.clone()]));

    assert_eq!(
        tracker.update(&id, "Title", "Body"),
        Err(TrackerError::Terminal {
            detail: "jira: update ENG-1 failed, response lost after send"
                .to_owned()
        })
    );
}

fn partitions_totally(outcome: &FetchOutcome, requested: &[ExternalId]) {
    let mut reported: Vec<&ExternalId> = outcome
        .found
        .iter()
        .map(|(id, _)| id)
        .chain(outcome.absent.iter())
        .chain(outcome.indeterminate.iter())
        .collect();
    let reported_count = reported.len();
    reported.sort_by_key(|id| id.as_str().to_owned());
    reported.dedup();
    assert_eq!(
        reported_count,
        reported.len(),
        "an id was reported more than once across the three vectors"
    );

    let mut expected: Vec<&ExternalId> = requested.iter().collect();
    expected.sort_by_key(|id| id.as_str().to_owned());
    expected.dedup();
    assert_eq!(
        reported, expected,
        "the partition does not cover exactly the requested ids"
    );
}

#[test]
fn a_bulk_fetch_partitions_every_requested_id_exactly_once() {
    let present = ExternalId::new("ENG-1".to_owned());
    let gone = ExternalId::new("ENG-9".to_owned());
    let unseen = ExternalId::new("ENG-7".to_owned());
    let requested = [present.clone(), gone.clone(), unseen.clone()];
    let tracker = FixedTracker::truncating(known(), vec![unseen.clone()]);

    let outcome = tracker
        .fetch_all(&requested)
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &requested);
    assert_eq!(
        outcome.found.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![&present]
    );
    assert_eq!(outcome.absent, vec![gone]);
    assert_eq!(outcome.indeterminate, vec![unseen]);
}

#[test]
fn a_duplicated_id_is_partitioned_once() {
    let id = ExternalId::new("ENG-1".to_owned());
    let tracker = FixedTracker::holding(known());

    let outcome = tracker
        .fetch_all(&[id.clone(), id.clone()])
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &[id]);
    assert_eq!(outcome.found.len(), 1);
}

#[test]
fn an_empty_request_makes_no_call_and_yields_an_empty_outcome() {
    let tracker = FixedTracker::holding(known());

    let outcome = tracker
        .fetch_all(&[])
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &[]);
}

#[test]
fn an_unprovable_id_is_indeterminate_rather_than_absent() {
    let unseen = ExternalId::new("ENG-7".to_owned());
    let requested = [unseen.clone()];
    let tracker = FixedTracker::truncating(Vec::new(), vec![unseen.clone()]);

    let outcome = tracker
        .fetch_all(&requested)
        .expect("the fake never fails a bulk fetch");

    partitions_totally(&outcome, &requested);
    assert!(
        outcome.absent.is_empty(),
        "a truncated fetch must not report absence"
    );
    assert_eq!(outcome.indeterminate, vec![unseen]);
}
