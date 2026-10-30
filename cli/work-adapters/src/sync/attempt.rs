//! One item's apply, tracking whether anything for it has landed so each
//! failure is classed by what a retry would repeat.

use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use tracker::ExternalId;
use tracker::TrackerError;

use crate::sync::apply::ApplyError;
use crate::sync::operation::SyncOperation;

/// Whether anything for an item had landed, locally or remotely, when it
/// failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemProgress {
    NothingLanded,
    MayHaveLanded,
}

/// The step after which a later failure is reported, as `after <step>: `.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    Pull,
    Push,
    Author,
    Link,
}

impl Landing {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Push => "push",
            Self::Author => "author",
            Self::Link => "link",
        }
    }
}

/// A step that refused, keeping its own class, or failed through its port.
pub enum StepError<E> {
    Refused(ApplyError),
    Failed(E),
}

/// A publishing step's failure; `FailedAfterPublish` means the content was
/// published before the step failed, so it may have landed.
pub enum PublishError<E> {
    Step(StepError<E>),
    FailedAfterPublish(E),
}

/// An apply's result, with the facts that outlive a failure: where the
/// item's local edits were kept and which issue a create made.
#[derive(Debug)]
pub struct Attempted<T> {
    pub result: Result<T, ApplyError>,
    pub kept_edits: Option<PathBuf>,
    pub created: Option<ExternalId>,
}

/// `observe_*` steps never count as landed; `land_*` steps count once they
/// succeed, and record the [`Landing`] later failures name.
pub struct ItemAttempt {
    item_id: String,
    progress: ItemProgress,
    created: Option<ExternalId>,
    last_landing: Option<Landing>,
    kept_edits: Option<PathBuf>,
    marker: Option<PathBuf>,
}

impl ItemAttempt {
    pub fn begin(item_id: &str) -> Self {
        Self {
            item_id: item_id.to_owned(),
            progress: ItemProgress::NothingLanded,
            created: None,
            last_landing: None,
            kept_edits: None,
            marker: None,
        }
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    /// The item now goes by `item_id`, as a file authored from a remote
    /// issue does once its id is allocated.
    pub fn known_as(&mut self, item_id: &str) {
        item_id.clone_into(&mut self.item_id);
    }

    /// The pending-push marker a create keeps, which every failure after
    /// the create, and every link failure, names.
    pub fn keeping_marker(&mut self, path: &Path) {
        self.marker = Some(path.to_path_buf());
    }

    pub fn observe_local<T, E: Display>(
        &self,
        operation: SyncOperation,
        effect: impl FnOnce() -> Result<T, StepError<E>>,
    ) -> Result<T, ApplyError> {
        effect().map_err(|error| self.classify_step_error(operation, error))
    }

    pub fn observe_tracker<T>(
        &self,
        operation: SyncOperation,
        effect: impl FnOnce() -> Result<T, TrackerError>,
    ) -> Result<T, ApplyError> {
        effect().map_err(|source| self.tracker_failure(operation, source))
    }

    pub fn land_local<T, E: Display>(
        &mut self,
        operation: SyncOperation,
        landed: Option<Landing>,
        effect: impl FnOnce() -> Result<T, PublishError<E>>,
    ) -> Result<T, ApplyError> {
        match effect() {
            Ok(value) => {
                self.landed(landed);
                Ok(value)
            }
            Err(PublishError::FailedAfterPublish(cause)) => {
                self.progress = ItemProgress::MayHaveLanded;
                Err(self.io_failure_without_landing(operation, cause))
            }
            Err(PublishError::Step(other)) => {
                Err(self.classify_step_error(operation, other))
            }
        }
    }

    pub fn land_tracker<T>(
        &mut self,
        operation: SyncOperation,
        landed: Option<Landing>,
        effect: impl FnOnce() -> Result<T, TrackerError>,
    ) -> Result<T, ApplyError> {
        let value = self.observe_tracker(operation, effect)?;
        self.landed(landed);
        Ok(value)
    }

    pub fn land_created(
        &mut self,
        effect: impl FnOnce() -> Result<ExternalId, TrackerError>,
    ) -> Result<ExternalId, TrackerError> {
        let key = effect()?;
        self.landed(None);
        self.created = Some(key.clone());
        Ok(key)
    }

    pub fn record_kept_edits(&mut self, copy: Option<PathBuf>) {
        self.kept_edits = copy;
    }

    pub fn finish<T>(self, result: Result<T, ApplyError>) -> Attempted<T> {
        Attempted {
            result,
            kept_edits: self.kept_edits,
            created: self.created,
        }
    }

    pub fn io_failure(
        &self,
        operation: SyncOperation,
        cause: impl Display,
    ) -> ApplyError {
        let after = self
            .last_landing
            .map(|landing| format!("after {}: ", landing.keyword()))
            .unwrap_or_default();
        self.io_error(operation, self.progress, &format!("{after}{cause}"))
    }

    pub fn tracker_failure(
        &self,
        operation: SyncOperation,
        source: TrackerError,
    ) -> ApplyError {
        ApplyError::Tracker {
            item_id: self.item_id.clone(),
            operation,
            source,
        }
    }

    const fn landed(&mut self, landing: Option<Landing>) {
        self.progress = ItemProgress::MayHaveLanded;
        if landing.is_some() {
            self.last_landing = landing;
        }
    }

    fn classify_step_error<E: Display>(
        &self,
        operation: SyncOperation,
        error: StepError<E>,
    ) -> ApplyError {
        match error {
            StepError::Refused(refusal) => refusal,
            StepError::Failed(cause) => self.io_failure(operation, cause),
        }
    }

    fn io_failure_without_landing(
        &self,
        operation: SyncOperation,
        cause: impl Display,
    ) -> ApplyError {
        self.io_error(operation, self.progress, &cause.to_string())
    }

    fn io_error(
        &self,
        operation: SyncOperation,
        progress: ItemProgress,
        reason: &str,
    ) -> ApplyError {
        let created = self
            .created
            .as_ref()
            .map(|key| format!("created {}; ", key.as_str()))
            .unwrap_or_default();
        let names_marker = operation != SyncOperation::MarkerWrite
            && (self.created.is_some()
                || operation == SyncOperation::LinkExternalId);
        let marker = self
            .marker
            .as_ref()
            .filter(|_| names_marker)
            .map(|path| format!(" (marker {})", path.display()))
            .unwrap_or_default();
        ApplyError::Io {
            item_id: self.item_id.clone(),
            operation,
            progress,
            detail: format!("{created}{reason}{marker}"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used)]
mod tests {
    use tracker::ExternalId;
    use tracker::TrackerError;

    use super::ItemAttempt;
    use super::ItemProgress;
    use super::Landing;
    use super::PublishError;
    use super::StepError;
    use crate::sync::apply::ApplyError;
    use crate::sync::operation::SyncOperation;

    fn io_parts(error: ApplyError) -> (ItemProgress, String) {
        match error {
            ApplyError::Io {
                progress, detail, ..
            } => (progress, detail),
            other => panic!("expected an Io failure, got {other:?}"),
        }
    }

    fn fail<T>() -> Result<T, StepError<&'static str>> {
        Err(StepError::Failed("injected"))
    }

    #[test]
    fn a_failure_before_anything_landed_is_nothing_landed() {
        let attempt = ItemAttempt::begin("0001");

        let error = attempt
            .observe_local(SyncOperation::ReadLocal, fail::<()>)
            .expect_err("the step failed");

        assert_eq!(
            io_parts(error),
            (ItemProgress::NothingLanded, "injected".to_owned())
        );
    }

    #[test]
    fn a_failure_after_a_landing_names_it_and_may_have_landed() {
        let mut attempt = ItemAttempt::begin("0001");
        attempt
            .land_local(SyncOperation::WriteLocal, Some(Landing::Pull), || {
                Ok::<(), PublishError<&str>>(())
            })
            .expect("the write landed");

        let error = attempt
            .observe_local(SyncOperation::BaselineSet, fail::<()>)
            .expect_err("the step failed");

        assert_eq!(
            io_parts(error),
            (
                ItemProgress::MayHaveLanded,
                "after pull: injected".to_owned()
            )
        );
    }

    #[test]
    fn a_failure_after_publish_may_have_landed_without_an_after_prefix() {
        let mut attempt = ItemAttempt::begin("0001");

        let error = attempt
            .land_local(SyncOperation::WriteLocal, Some(Landing::Pull), || {
                Err::<(), _>(PublishError::FailedAfterPublish("not durable"))
            })
            .expect_err("the write was not durable");

        assert_eq!(
            io_parts(error),
            (ItemProgress::MayHaveLanded, "not durable".to_owned())
        );
    }

    #[test]
    fn a_none_landing_keeps_the_previous_prefix() {
        let mut attempt = ItemAttempt::begin("0001");
        attempt
            .land_local(SyncOperation::WriteLocal, None, || {
                Ok::<(), PublishError<&str>>(())
            })
            .expect("the clear landed");
        let unprefixed = attempt
            .observe_local(SyncOperation::HashLocal, fail::<()>)
            .expect_err("the step failed");
        attempt
            .land_local(
                SyncOperation::LinkExternalId,
                Some(Landing::Link),
                || Ok::<(), PublishError<&str>>(()),
            )
            .expect("the link landed");
        attempt
            .land_local(SyncOperation::MarkerWrite, None, || {
                Ok::<(), PublishError<&str>>(())
            })
            .expect("the marker landed");

        let error = attempt
            .observe_local(SyncOperation::BaselineSet, fail::<()>)
            .expect_err("the step failed");

        assert_eq!(io_parts(unprefixed).1, "injected");
        assert_eq!(io_parts(error).1, "after link: injected");
    }

    #[test]
    fn a_refusal_passes_through_with_its_own_class() {
        let attempt = ItemAttempt::begin("0001");

        let error = attempt
            .observe_local(SyncOperation::RecheckLocal, || {
                Err::<(), StepError<&str>>(StepError::Refused(
                    ApplyError::LocalChangedSinceClassified {
                        item_id: "0001".to_owned(),
                    },
                ))
            })
            .expect_err("the step refused");

        assert!(matches!(
            error,
            ApplyError::LocalChangedSinceClassified { .. }
        ));
    }

    #[test]
    fn every_failure_after_a_create_names_the_key_and_the_marker() {
        let mut attempt = ItemAttempt::begin("0001");
        attempt.keeping_marker(std::path::Path::new("pending-push/0001.json"));
        attempt
            .land_created(|| Ok(ExternalId::new("PP-12".to_owned())))
            .expect("the create landed");
        attempt
            .land_local(
                SyncOperation::LinkExternalId,
                Some(Landing::Link),
                || Ok::<(), PublishError<&str>>(()),
            )
            .expect("the link landed");

        let error = attempt
            .observe_local(SyncOperation::BaselineSet, fail::<()>)
            .expect_err("the step failed");

        assert_eq!(
            io_parts(error),
            (
                ItemProgress::MayHaveLanded,
                "created PP-12; after link: injected (marker \
                 pending-push/0001.json)"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_tracker_failure_keeps_the_trackers_error() {
        let attempt = ItemAttempt::begin("0001");

        let error = attempt
            .observe_tracker(SyncOperation::Show, || {
                Err::<(), _>(TrackerError::Retryable {
                    detail: "offline".to_owned(),
                })
            })
            .expect_err("the read failed");

        assert!(matches!(
            error,
            ApplyError::Tracker {
                operation: SyncOperation::Show,
                source: TrackerError::Retryable { .. },
                ..
            }
        ));
    }
}
