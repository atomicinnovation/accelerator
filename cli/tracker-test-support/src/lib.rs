//! A recording fake for `tracker::RemoteTracker`, plus the contract harness
//! in [`contract`] that every implementation — this fake included — must
//! satisfy.
//!
//! The `ContractSubject` accessors panic when a subject was not configured
//! for the condition they supply: that is a test-setup mistake in the
//! caller, not a runtime condition to recover from.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

pub mod contract;
pub mod evidence;

use std::cell::RefCell;

use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::TrackerError;

/// One call the fake observed, carrying enough of the request for a test to
/// assert the whole-content contract rather than only the call count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Create {
        title: String,
        body: String,
        kind: String,
    },
    Update {
        id: ExternalId,
        title: String,
        body: String,
    },
    Show {
        id: ExternalId,
    },
    FetchAll {
        ids: Vec<ExternalId>,
    },
}

/// A `RemoteTracker` fake that records every call it receives and can be
/// configured to fail in the shapes the sync engine must survive.
pub struct RecordingTracker {
    issues: RefCell<Vec<(ExternalId, RemoteIssue)>>,
    unprovable: Vec<ExternalId>,
    show_failures: Vec<(ExternalId, TrackerError)>,
    update_failures: Vec<(ExternalId, TrackerError)>,
    create_failure: Option<(TrackerError, bool)>,
    next_id: RefCell<u32>,
    calls: RefCell<Vec<Call>>,
}

impl RecordingTracker {
    #[must_use]
    pub const fn holding(issues: Vec<(ExternalId, RemoteIssue)>) -> Self {
        Self {
            issues: RefCell::new(issues),
            unprovable: Vec::new(),
            show_failures: Vec::new(),
            update_failures: Vec::new(),
            create_failure: None,
            next_id: RefCell::new(1),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// A tracker whose bulk retrieval cannot account for `unprovable`: those
    /// ids come back `indeterminate`, never `absent`.
    #[must_use]
    pub fn truncating(
        issues: Vec<(ExternalId, RemoteIssue)>,
        unprovable: Vec<ExternalId>,
    ) -> Self {
        Self {
            unprovable,
            ..Self::holding(issues)
        }
    }

    /// A tracker whose write to any id in `lossy` goes unacknowledged: the
    /// fake cannot say whether the mutation applied, so it reports
    /// `Terminal`.
    #[must_use]
    pub fn losing(
        issues: Vec<(ExternalId, RemoteIssue)>,
        lossy: Vec<ExternalId>,
    ) -> Self {
        let mut tracker = Self::holding(issues);
        for id in lossy {
            let detail =
                format!("fake: update {id} failed, response lost after send");
            tracker
                .update_failures
                .push((id, TrackerError::Terminal { detail }));
        }
        tracker
    }

    #[must_use]
    pub fn failing_update(
        mut self,
        id: ExternalId,
        error: TrackerError,
    ) -> Self {
        self.update_failures.push((id, error));
        self
    }

    #[must_use]
    pub fn failing_create(mut self, error: TrackerError) -> Self {
        self.create_failure = Some((error, false));
        self
    }

    /// The terminal-failure-that-in-fact-succeeded shape: the issue is
    /// recorded as created, then the call reports `error`.
    #[must_use]
    pub fn creating_then_failing(mut self, error: TrackerError) -> Self {
        self.create_failure = Some((error, true));
        self
    }

    #[must_use]
    pub fn failing_show(mut self, id: ExternalId, error: TrackerError) -> Self {
        self.show_failures.push((id, error));
        self
    }

    #[must_use]
    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }

    fn issue(&self, id: &ExternalId) -> Option<RemoteIssue> {
        self.issues
            .borrow()
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, issue)| issue.clone())
    }

    fn allocate_id(&self) -> ExternalId {
        let mut next = self.next_id.borrow_mut();
        let id = ExternalId::new(format!("REC-{next}"));
        *next += 1;
        id
    }
}

impl RemoteTracker for RecordingTracker {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        self.calls.borrow_mut().push(Call::Create {
            title: title.to_owned(),
            body: body.to_owned(),
            kind: kind.to_owned(),
        });

        if let Some((error, also_creates)) = &self.create_failure {
            if *also_creates {
                let id = self.allocate_id();
                self.issues.borrow_mut().push((
                    id,
                    RemoteIssue {
                        updated: RemoteTimestamp::NotReported,
                        body: format!("{title}\n{body}"),
                    },
                ));
            }
            return Err(error.clone());
        }

        let id = self.allocate_id();
        self.issues.borrow_mut().push((
            id.clone(),
            RemoteIssue {
                updated: RemoteTimestamp::NotReported,
                body: format!("{title}\n{body}"),
            },
        ));
        Ok(id)
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        self.calls.borrow_mut().push(Call::Update {
            id: id.clone(),
            title: title.to_owned(),
            body: body.to_owned(),
        });

        if let Some((_, error)) =
            self.update_failures.iter().find(|(known, _)| known == id)
        {
            return Err(error.clone());
        }

        let mut issues = self.issues.borrow_mut();
        match issues.iter_mut().find(|(known, _)| known == id) {
            Some(entry) => {
                entry.1.body = format!("{title}\n{body}");
                Ok(())
            }
            None => Err(TrackerError::Retryable {
                detail: format!("fake: update {id} rejected, no such issue"),
            }),
        }
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.calls.borrow_mut().push(Call::Show { id: id.clone() });

        if let Some((_, error)) =
            self.show_failures.iter().find(|(known, _)| known == id)
        {
            return Err(error.clone());
        }

        self.issue(id).ok_or_else(|| TrackerError::Retryable {
            detail: format!("fake: show {id} failed, connection refused"),
        })
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        self.calls
            .borrow_mut()
            .push(Call::FetchAll { ids: ids.to_vec() });

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
                    outcome.found.push((id.clone(), issue.updated));
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

impl contract::ContractSubject for RecordingTracker {
    fn tracker(&self) -> &dyn RemoteTracker {
        self
    }

    fn unaccountable_id(&self) -> ExternalId {
        self.unprovable
            .first()
            .cloned()
            .expect("configure this subject with `truncating` first")
    }

    fn unreadable_id(&self) -> ExternalId {
        self.show_failures
            .first()
            .map(|(id, _)| id.clone())
            .expect("configure this subject with `failing_show` first")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn issue(stamp: &str, body: &str) -> RemoteIssue {
        RemoteIssue {
            updated: RemoteTimestamp::Reported(stamp.to_owned()),
            body: body.to_owned(),
        }
    }

    #[test]
    fn truncating_omits_the_ids_it_claims_to() {
        let seen = ExternalId::new("REC-1".to_owned());
        let unseen = ExternalId::new("REC-2".to_owned());
        let tracker = RecordingTracker::truncating(
            vec![(seen.clone(), issue("t", "body"))],
            vec![unseen.clone()],
        );

        let outcome = tracker
            .fetch_all(&[seen.clone(), unseen.clone()])
            .expect("fetch_all never fails against this fake");

        assert_eq!(
            outcome.found.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            vec![&seen]
        );
        assert!(outcome.absent.is_empty());
        assert_eq!(outcome.indeterminate, vec![unseen]);
    }

    #[test]
    fn losing_drops_the_write() {
        let id = ExternalId::new("REC-1".to_owned());
        let tracker = RecordingTracker::losing(
            vec![(id.clone(), issue("t", "body"))],
            vec![id.clone()],
        );

        let result = tracker.update(&id, "New title", "New body");

        assert_eq!(
            result,
            Err(TrackerError::Terminal {
                detail: format!(
                    "fake: update {id} failed, response lost after send"
                )
            })
        );
    }

    #[test]
    fn calls_records_each_operation_once_and_in_order() {
        let id = ExternalId::new("REC-1".to_owned());
        let tracker =
            RecordingTracker::holding(vec![(id.clone(), issue("t", "body"))]);

        tracker
            .create("Title", "Body", "story")
            .expect("create succeeds against this fake");
        tracker
            .update(&id, "New title", "New body")
            .expect("update succeeds against this fake");
        tracker.show(&id).expect("show succeeds against this fake");
        tracker
            .fetch_all(std::slice::from_ref(&id))
            .expect("fetch_all never fails against this fake");

        let calls = tracker.calls();
        assert_eq!(calls.len(), 4);
        assert!(matches!(calls[0], Call::Create { .. }));
        assert!(matches!(calls[1], Call::Update { .. }));
        assert!(matches!(calls[2], Call::Show { .. }));
        assert!(matches!(calls[3], Call::FetchAll { .. }));
    }

    #[test]
    fn each_failure_seam_returns_the_class_it_was_built_with() {
        let update_target = ExternalId::new("REC-1".to_owned());
        let show_target = ExternalId::new("REC-2".to_owned());
        let retryable = TrackerError::Retryable {
            detail: "boom".to_owned(),
        };
        let terminal = TrackerError::Terminal {
            detail: "boom".to_owned(),
        };

        let update_failing = RecordingTracker::holding(vec![(
            update_target.clone(),
            issue("t", "body"),
        )])
        .failing_update(update_target.clone(), retryable.clone());
        assert_eq!(
            update_failing.update(&update_target, "t", "b"),
            Err(retryable.clone())
        );

        let show_failing = RecordingTracker::holding(Vec::new())
            .failing_show(show_target.clone(), terminal.clone());
        assert_eq!(show_failing.show(&show_target), Err(terminal.clone()));

        let create_failing = RecordingTracker::holding(Vec::new())
            .failing_create(retryable.clone());
        assert_eq!(create_failing.create("t", "b", "story"), Err(retryable));
        assert!(create_failing.calls().len() == 1);

        let creating_then_failing = RecordingTracker::holding(Vec::new())
            .creating_then_failing(terminal.clone());
        assert_eq!(
            creating_then_failing.create("t", "b", "story"),
            Err(terminal)
        );
        let outcome = creating_then_failing
            .fetch_all(&[ExternalId::new("REC-1".to_owned())])
            .expect("fetch_all never fails against this fake");
        assert_eq!(outcome.found.len(), 1);
    }
}
