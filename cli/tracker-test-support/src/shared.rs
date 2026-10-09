use std::rc::Rc;

use tracker::CreatePreview;
use tracker::Discovery;
use tracker::ExternalId;
use tracker::FetchOutcome;
use tracker::Located;
use tracker::RemoteIssue;
use tracker::RemoteTracker;
use tracker::ScopeError;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;
use tracker::VisibleEntity;

use crate::RecordingTracker;

/// Shares one `RecordingTracker` between code that takes ownership of a
/// `Box<dyn RemoteTracker>` and the test that inspects the call log
/// afterwards.
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

    fn locate(&self, id: &ExternalId) -> Result<Located, TrackerError> {
        self.0.locate(id)
    }

    fn fetch_all(
        &self,
        ids: &[ExternalId],
    ) -> Result<FetchOutcome, TrackerError> {
        self.0.fetch_all(ids)
    }

    fn search(&self, scope: &SearchScope) -> Result<Discovery, TrackerError> {
        self.0.search(scope)
    }

    fn resolve_scope(
        &self,
        scope: &SearchScope,
    ) -> Result<SearchScope, ScopeError> {
        self.0.resolve_scope(scope)
    }

    fn enumerate_visible_entities(
        &self,
    ) -> Result<Vec<VisibleEntity>, TrackerError> {
        self.0.enumerate_visible_entities()
    }

    fn preview_create(
        &self,
        kind: &str,
    ) -> Result<CreatePreview, TrackerError> {
        self.0.preview_create(kind)
    }

    fn validate_update(
        &self,
        id: &ExternalId,
        title: &str,
        body: &str,
    ) -> ValidationOutcome {
        self.0.validate_update(id, title, body)
    }
}
