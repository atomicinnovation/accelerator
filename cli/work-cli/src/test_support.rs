//! Test doubles shared by this binary's unit tests.

use std::path::Path;
use std::rc::Rc;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::AtomicWrite;
use corpus_adapters::FileCorpusStore;

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

pub struct FakeConfig(pub std::collections::HashMap<String, String>);

impl ConfigAccess for FakeConfig {
    fn get(
        &self,
        key: &::config::Key,
        _level: Option<::config::Level>,
    ) -> Result<::config::Resolved, ::config::ConfigError> {
        Ok(self.0.get(&key.to_string()).map_or(
            ::config::Resolved::Absent,
            |value| {
                ::config::Resolved::Found(::config::Value::Scalar(
                    ::config::Scalar::String(value.clone()),
                ))
            },
        ))
    }

    fn set(
        &self,
        _key: &::config::Key,
        _value: &str,
        _level: ::config::Level,
    ) -> Result<(), ::config::ConfigError> {
        unreachable!("create never writes config")
    }
}

pub struct PluginWorkItemTemplate;

impl ReadTemplate for PluginWorkItemTemplate {
    fn resolve_template(
        &self,
        _name: &str,
        _config_path: Option<&str>,
        _templates_dir: &str,
    ) -> Result<Option<::config::ResolvedTemplate>, ::config::ConfigError> {
        self.plugin_default("work-item")
    }

    fn template_names(&self) -> Result<Vec<String>, ::config::ConfigError> {
        Ok(vec!["work-item".to_owned()])
    }

    fn plugin_default(
        &self,
        _name: &str,
    ) -> Result<Option<::config::ResolvedTemplate>, ::config::ConfigError> {
        Ok(Some(::config::ResolvedTemplate {
            source: ::config::TemplateSource::PluginDefault,
            abs_path: "templates/work-item.md".to_owned(),
            display_path: "templates/work-item.md".to_owned(),
            content: include_str!("../../../templates/work-item.md").to_owned(),
            warning: None,
        }))
    }
}

/// A store whose operations on the paths `applies` selects fail, `left`
/// times in all.
pub struct Faults {
    pub inner: FileCorpusStore,
    pub applies: fn(&Path) -> bool,
    pub left: std::cell::Cell<usize>,
}

impl Faults {
    fn check(&self, path: &Path) -> Result<(), corpus::StoreError> {
        if (self.applies)(path) && self.left.get() > 0 {
            self.left.set(self.left.get() - 1);
            return Err(corpus::StoreError::Io {
                path: path.display().to_string(),
                detail: "injected".to_owned(),
            });
        }
        Ok(())
    }
}

impl AtomicWrite for Faults {
    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), corpus::StoreError> {
        self.check(path)?;
        self.inner.write(path, bytes)
    }
}

impl ExclusiveCreate for Faults {
    fn create_new(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), corpus::StoreError> {
        self.check(path)?;
        self.inner.create_new(path, bytes)
    }
}

impl FileRemove for Faults {
    fn remove(&self, path: &Path) -> Result<(), corpus::StoreError> {
        self.check(path)?;
        self.inner.remove(path)
    }
}
