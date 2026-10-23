//! Test doubles shared by this binary's unit tests.

use std::path::Path;
use std::rc::Rc;

use ::config::ConfigAccess;
use ::config::ReadTemplate;
use corpus::store::ExclusiveCreate;
use corpus::store::FileRemove;
use corpus::AtomicWrite;
use corpus_adapters::FileCorpusStore;

use tracker::RemoteTracker;
use tracker_test_support::RecordingTracker;
use tracker_test_support::SharedTracker;

use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

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

/// A store whose operations on the paths `applies` selects fail with
/// `error`, `left` times in all.
pub struct Faults<S = FileCorpusStore> {
    pub inner: S,
    pub applies: fn(&Path) -> bool,
    pub left: std::cell::Cell<usize>,
    pub error: fn(&Path) -> corpus::StoreError,
}

/// The fault a store reports when a test does not say which.
pub fn injected(path: &Path) -> corpus::StoreError {
    corpus::StoreError::Io {
        path: path.display().to_string(),
        detail: "injected".to_owned(),
    }
}

impl<S> Faults<S> {
    fn check(&self, path: &Path) -> Result<(), corpus::StoreError> {
        if (self.applies)(path) && self.left.get() > 0 {
            self.left.set(self.left.get() - 1);
            return Err((self.error)(path));
        }
        Ok(())
    }
}

impl<S: AtomicWrite + ?Sized> AtomicWrite for Faults<Box<S>> {
    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), corpus::StoreError> {
        self.check(path)?;
        self.inner.write(path, bytes)
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

impl<S: FileRemove + ?Sized> FileRemove for Faults<Box<S>> {
    fn remove(&self, path: &Path) -> Result<(), corpus::StoreError> {
        self.check(path)?;
        self.inner.remove(path)
    }
}

/// Leaves a re-key run plan in progress in the repository at `root`.
pub fn rekey_in_progress(root: &Path) {
    use work::rekey::run_plan::InProgressPlan;
    use work::rekey::run_plan::RunPlan;
    use work::rekey::write_set::WriteSet;

    save_rekey_plan(
        root,
        &RunPlan::InProgress(InProgressPlan::new(
            empty_fingerprint(),
            Vec::new(),
            WriteSet::default(),
        )),
    );
}

/// Leaves a previewed re-key run plan in the repository at `root`.
pub fn rekey_proposed(root: &Path) {
    use work::rekey::run_plan::ProposedPlan;
    use work::rekey::run_plan::RunPlan;

    save_rekey_plan(
        root,
        &RunPlan::Proposed(ProposedPlan::new(empty_fingerprint(), Vec::new())),
    );
}

fn empty_fingerprint() -> work::rekey::fingerprint::IdentityFingerprint {
    work::rekey::fingerprint::IdentityFingerprint::of(
        &corpus::WorkItemIdScheme::numeric(),
        &[],
    )
}

fn save_rekey_plan(root: &Path, plan: &work::rekey::run_plan::RunPlan) {
    use work_adapters::rekey::run_plan_store::FileRunPlanStore;
    use work_adapters::rekey::run_plan_store::RunPlanStore as _;

    let store = FileCorpusStore::new(root);
    let saved = FileRunPlanStore::new(
        &root.join(crate::sync::STATE_DIR),
        root,
        &store,
        &store,
    )
    .save(plan);
    assert!(saved.is_ok(), "{saved:?}");
}

/// Leaves a re-key run plan no reader can parse in the repository at
/// `root`.
pub fn rekey_plan_unreadable(root: &Path) {
    let dir = root.join(crate::sync::STATE_DIR).join("rekey");
    let written = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join(".gitignore"), "*\n"))
        .and_then(|()| std::fs::write(dir.join("run-plan.json"), "{"));
    assert!(written.is_ok(), "{written:?}");
}

/// Every file under `root`'s `meta/`, by path, to show a refusal wrote
/// nothing.
#[must_use]
pub fn meta_tree(root: &Path) -> std::collections::BTreeMap<String, String> {
    fn collect(
        root: &Path,
        dir: &Path,
        tree: &mut std::collections::BTreeMap<String, String>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, tree);
            } else if let (Ok(relative), Ok(content)) =
                (path.strip_prefix(root), std::fs::read_to_string(&path))
            {
                tree.insert(relative.display().to_string(), content);
            }
        }
    }
    let mut tree = std::collections::BTreeMap::new();
    collect(root, &root.join("meta"), &mut tree);
    tree
}

/// An applied run's report with nothing in it, for a test to fill in the
/// fields it is about.
pub fn run_report() -> work_adapters::sync::run::RunReport {
    work_adapters::sync::run::RunReport {
        scope: work_adapters::sync::report::CorpusScope::WholeCorpus,
        mode: work_adapters::sync::run::RunMode::Apply,
        inventory: work::inventory::CorpusInventory::default(),
        identity: Vec::new(),
        promotions: Vec::new(),
        deferred: 0,
        reported: Vec::new(),
        unclassifiable: Vec::new(),
        refused_orders: Vec::new(),
        read_failure: None,
        baseline_degradation: work_adapters::sync::baseline::Degradation::None,
        finalisation: work_adapters::sync::report::Finalisation::Finalised,
        dossiers: Vec::new(),
        discovery: work_adapters::sync::run::DiscoveryStatus::Ran { found: 0 },
        discovery_deferral: None,
        keyed_read_budget_limited: false,
    }
}
