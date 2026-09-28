//! Test doubles shared by this binary's unit tests.

use std::rc::Rc;

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTracker;
use tracker::TrackerError;
use tracker_test_support::RecordingTracker;

use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

/// Shares one `RecordingTracker` between the registry (which hands the
/// engine a `Box<dyn RemoteTracker>`) and the test (which inspects the call
/// log afterwards), since the box is moved into `run_sync`.
pub struct SharedTracker(pub Rc<RecordingTracker>);

impl RemoteTracker for SharedTracker {
    fn create(
        &self,
        title: &str,
        body: &str,
        kind: &str,
    ) -> Result<ExternalId, TrackerError> {
        self.0.create(title, body, kind)
    }

    fn update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> Result<(), TrackerError> {
        self.0.update(id, title, body)
    }

    fn show(&self, id: &ExternalId) -> Result<RemoteIssue, TrackerError> {
        self.0.show(id)
    }

    fn locate(
        &self,
        id: &ExternalId,
    ) -> Result<tracker::Located, TrackerError> {
        self.0.locate(id)
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<tracker::FetchOutcome, TrackerError> {
        self.0.fetch_all(ids)
    }

    fn search(
        &self,
        scope: &tracker::SearchScope,
    ) -> Result<tracker::Discovery, TrackerError> {
        self.0.search(scope)
    }

    fn resolve_scope(
        &self,
        scope: &tracker::SearchScope,
    ) -> Result<tracker::SearchScope, tracker::ScopeError> {
        self.0.resolve_scope(scope)
    }

    fn enumerate_visible_entities(
        &self,
    ) -> Result<Vec<tracker::VisibleEntity>, tracker::TrackerError> {
        self.0.enumerate_visible_entities()
    }

    fn preview_create(
        &self,
        kind: &str,
    ) -> Result<tracker::CreatePreview, TrackerError> {
        self.0.preview_create(kind)
    }

    fn validate_update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> tracker::ValidationOutcome {
        self.0.validate_update(id, title, body)
    }
}

pub struct StubRegistry(pub Rc<RecordingTracker>);

impl TrackerRegistry for StubRegistry {
    fn resolve(
        &self,
        _name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
        Ok(Box::new(SharedTracker(Rc::clone(&self.0))))
    }
}
