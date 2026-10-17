//! `accelerator work rekey`: converges every work item onto the configured
//! `work.id_pattern`, finishing an interrupted run first; previews the
//! mapping for a later apply; or discards a proposed run, or one a revert
//! has undone.

use std::cell::Cell;
use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus::WorkItemIdScheme;
use tracker::ExternalId;
use vcs::guarded_working_copy::GUARDED_SCOPES;
use vcs::RepositoryProbe;
use work::rekey::entry::RekeyEntry;
use work::rekey::mapping::RefreshedKeys;
use work::rekey::outcome::unreverted_listing;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work_adapters::creation::CreationStore;
use work_adapters::draft_id::RandomSuffixDraws;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::identity_run::waiting_line;
use work_adapters::identity_run_lock::IdentityRunLock;
use work_adapters::rekey::execute::RekeyCorpus;
use work_adapters::rekey::guards::PendingPushDir;
use work_adapters::rekey::guards::SyncState;
use work_adapters::rekey::refresh::refresh_external_ids;
use work_adapters::rekey::run::Forcing;
use work_adapters::rekey::run::KeyRefresh;
use work_adapters::rekey::run::RekeyAction;
use work_adapters::rekey::run::RekeyPorts;
use work_adapters::rekey::run::RekeyProgress;
use work_adapters::rekey::run::RekeyReport;
use work_adapters::rekey::run::RekeyRequest;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::fetch::RetrievalStrategy;

use crate::cli::RekeyArgs;
use crate::exit_codes;
use crate::identity_workspace::CorpusWorkspace;
use crate::identity_workspace::IdentityWorkspace;
use crate::tracker_registry::SelectionError;
use crate::tracker_registry::TrackerRegistry;

/// Where a run's lines go: stdout for what changed, stderr for the rest.
pub trait Console {
    fn out(&self, line: &str);
    fn err(&self, line: &str);
}

pub struct Terminal;

impl Console for Terminal {
    fn out(&self, line: &str) {
        println!("{line}");
    }

    fn err(&self, line: &str) {
        eprintln!("{line}");
    }
}

/// What a run reads from outside its configuration.
pub struct RekeyEnvironment<'a> {
    pub forcing: Forcing,
    pub repository: &'a dyn RepositoryProbe,
    pub store_at: &'a dyn Fn(&Path) -> Box<dyn CreationStore>,
    pub registry: &'a dyn TrackerRegistry,
    pub console: &'a dyn Console,
}

struct Shown<'a>(&'a dyn Console);

impl RekeyProgress for Shown<'_> {
    fn waiting(&self, holder: Option<&Holder>) {
        self.0.err(&waiting_line(holder));
    }

    fn applied(&self, entry: &RekeyEntry, path: &str) {
        self.0
            .out(&format!("{} → {}\t{path}", entry.old, entry.target.id()));
    }

    fn remaining(&self, lines: &[String]) {
        self.0.err("the run in progress has still to re-key:");
        for line in lines {
            self.0.out(line);
        }
    }

    fn partly_applied(&self, items: &[PathBuf], recoveries: &[PathBuf]) {
        if items.is_empty() && recoveries.is_empty() {
            self.0.err(
                "no item or recovery directory shows a half-applied entry",
            );
            return;
        }
        self.0.err("evidence of a half-applied entry:");
        for item in items {
            self.0.err(&format!("  item: {}", item.display()));
        }
        for recovery in recoveries {
            self.0.err(&format!("  recovery: {}", recovery.display()));
        }
    }
}

/// Resolves the configured tracker only when a run first asks for keys,
/// keeping why it could not, since a selection failure exits with its own
/// code rather than as a re-key refusal.
struct ConfiguredRefresh<'a> {
    registry: &'a dyn TrackerRegistry,
    integration: &'a str,
    strategy: RetrievalStrategy,
    asked: Cell<bool>,
    unselectable: RefCell<Option<SelectionError>>,
}

impl KeyRefresh for ConfiguredRefresh<'_> {
    fn refresh(
        &self,
        keys: &[ExternalId],
    ) -> Result<RefreshedKeys, NotRekeyed> {
        self.asked.set(true);
        if keys.is_empty() {
            return Ok(RefreshedKeys::default());
        }
        match self.registry.resolve(self.integration) {
            Ok(tracker) => {
                refresh_external_ids(keys, tracker.as_ref(), self.strategy)
            }
            Err(selection) => {
                let message = selection.message();
                *self.unselectable.borrow_mut() = Some(selection);
                Err(NotRekeyed::TrackerUnreachable(message))
            }
        }
    }
}

const fn skipped_by_force(action: RekeyAction) -> &'static str {
    match action {
        RekeyAction::Apply => {
            "warning: ACCELERATOR_FORCE is set: skipping the working-copy \
             check, so version control may not be able to undo this run"
        }
        RekeyAction::Preview => {
            "warning: ACCELERATOR_FORCE is set: an apply will skip the \
             working-copy check, so version control may not be able to undo \
             it"
        }
        RekeyAction::Discard => {
            "warning: ACCELERATOR_FORCE is set: a run plan that cannot be \
             read, or whose paths have changed, is set aside rather than \
             refused"
        }
    }
}

/// Where the repository keeps its corpus and its identity state, and the
/// pattern every item is to match.
struct Located {
    root: PathBuf,
    work_dir: PathBuf,
    scheme: WorkItemIdScheme,
    integration: Option<String>,
    integrations_root: PathBuf,
}

fn locate(
    start: &Path,
    config: &dyn ConfigAccess,
) -> Result<Located, kernel::Error> {
    let root = config_adapters::FileConfigStore::discover_root(start);
    Ok(Located {
        work_dir: crate::config::resolve_work_dir(config, &root)?,
        scheme: crate::config::resolve_scheme(config)?,
        integration: crate::config::configured_override(
            config,
            "work.integration",
        )?,
        integrations_root: crate::sync::integrations_dir(config, &root)?,
        root,
    })
}

enum Workspace {
    Integrated(IdentityWorkspace),
    Local(CorpusWorkspace),
}

impl Workspace {
    fn open(
        config: &dyn ConfigAccess,
        located: &Located,
    ) -> Result<Self, String> {
        located.integration.as_deref().map_or_else(
            || {
                CorpusWorkspace::open(config, &located.root, &located.work_dir)
                    .map(Self::Local)
            },
            |integration| {
                IdentityWorkspace::open(
                    config,
                    &located.root,
                    &located.work_dir,
                    &located.integrations_root,
                    integration,
                )
                .map(Self::Integrated)
            },
        )
    }

    const fn corpus(&self) -> &CorpusWorkspace {
        match self {
            Self::Integrated(workspace) => workspace.corpus(),
            Self::Local(corpus) => corpus,
        }
    }

    const fn integrated(&self) -> Option<&IdentityWorkspace> {
        match self {
            Self::Integrated(workspace) => Some(workspace),
            Self::Local(_) => None,
        }
    }
}

/// The accelerator state scopes and every corpus root, repo-relative.
fn guarded_scopes(root: &Path, corpus: &CorpusWorkspace) -> Vec<String> {
    GUARDED_SCOPES
        .iter()
        .map(|scope| (*scope).to_owned())
        .chain(corpus.roots().iter().filter_map(|corpus_root| {
            corpus_root
                .strip_prefix(root)
                .ok()
                .map(|relative| format!("{}/", relative.display()))
        }))
        .collect()
}

#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    args: &RekeyArgs,
    environment: &RekeyEnvironment<'_>,
) -> u8 {
    let console = environment.console;
    let internal = |message: String| {
        console.err(&format!("Error: {message}"));
        exit_codes::ERROR
    };
    let action = if args.preview {
        RekeyAction::Preview
    } else if args.discard {
        RekeyAction::Discard
    } else {
        RekeyAction::Apply
    };
    if environment.forcing == Forcing::Forced {
        console.err(skipped_by_force(action));
    }
    let located = match locate(start, config) {
        Ok(located) => located,
        Err(error) => return internal(error.to_string()),
    };
    let workspace = match Workspace::open(config, &located) {
        Ok(workspace) => workspace,
        Err(message) => return internal(message),
    };
    let request = RekeyRequest {
        action,
        forcing: environment.forcing,
        scheme: located.scheme.clone(),
    };
    let refresh = ConfiguredRefresh {
        registry: environment.registry,
        integration: located.integration.as_deref().unwrap_or_default(),
        strategy: if args.per_item_reads {
            RetrievalStrategy::PerItem
        } else {
            RetrievalStrategy::Bulk
        },
        asked: Cell::new(false),
        unselectable: RefCell::new(None),
    };
    let outcome = rekey(&request, &located, &workspace, environment, &refresh);
    if args.per_item_reads && !refresh.asked.get() {
        console.err(
            "note: --per-item-reads was ignored: it applies only when a \
             `{tracker}` mapping is computed",
        );
    }
    if let Some(selection) = refresh.unselectable.take() {
        console.err(&format!("Error: {}", selection.message()));
        return crate::create::dispatch_code_for_selection_error(&selection);
    }
    match outcome {
        Ok(report) => {
            render(&report, console);
            exit_codes::CLEAN
        }
        Err(refusal) => {
            console.err(&format!("Error: {refusal}"));
            refusal.exit_code()
        }
    }
}

fn rekey(
    request: &RekeyRequest,
    located: &Located,
    workspace: &Workspace,
    environment: &RekeyEnvironment<'_>,
    refresh: &dyn KeyRefresh,
) -> Result<RekeyReport, NotRekeyed> {
    let root = &located.root;
    let corpus = workspace.corpus();
    let project_store = (environment.store_at)(root);
    let baseline_store = workspace.integrated().map(|integrated| {
        let path = integrated.baseline_path();
        let dir = path
            .parent()
            .map_or_else(|| root.clone(), Path::to_path_buf);
        (environment.store_at)(&dir)
    });
    let baseline: Option<BaselineStore<'_>> = workspace
        .integrated()
        .zip(baseline_store.as_deref())
        .map(|(integrated, store)| integrated.baseline(store));
    let identity_run =
        IdentityRunLock::new(corpus.state_dir(), project_store.as_ref());
    let plans = FileRunPlanStore::new(
        corpus.state_dir(),
        root,
        project_store.as_ref(),
        project_store.as_ref(),
    );
    let records = corpus.retirement_records(project_store.as_ref());
    let items = FilesystemWorkItemFiles::new(&located.work_dir);
    let scopes = guarded_scopes(root, corpus);
    let progress = Shown(environment.console);
    work_adapters::rekey::run::run(
        request,
        &RekeyPorts {
            identity_run: &identity_run,
            store: &plans,
            corpus: RekeyCorpus {
                repo_root: root,
                state_dir: corpus.state_dir(),
                ports: corpus.retirement_ports(
                    project_store.as_ref(),
                    baseline.as_ref(),
                ),
            },
            items: &items,
            sync: SyncState {
                state_dir: corpus.state_dir(),
                records: &records,
                recovery: corpus.recovery(),
                pending_push: workspace.integrated().map(|integrated| {
                    PendingPushDir {
                        integrations_root: integrated.integrations_root(),
                        integration: integrated.integration(),
                    }
                }),
            },
            repository: environment.repository,
            scopes: &scopes,
            progress: &progress,
            refresh,
            draws: &RefCell::new(RandomSuffixDraws),
        },
    )
}

fn render(report: &RekeyReport, console: &dyn Console) {
    match report {
        RekeyReport::AlreadyConverged => {
            console.err("every work item already matches `work.id_pattern`");
        }
        RekeyReport::Previewed { lines, dirty } => {
            for line in lines {
                console.out(line);
            }
            if !dirty.is_empty() {
                console.err(
                    "warning: the working copy has uncommitted changes; \
                     `accelerator work rekey` will refuse until they are \
                     committed or reverted:",
                );
                for path in dirty {
                    console.err(&format!("  {path}"));
                }
            }
        }
        RekeyReport::Rekeyed { .. } => {}
        RekeyReport::NothingToDiscard => {
            console.err("no re-key run plan to discard");
        }
        RekeyReport::Discarded => console.err("discarded the re-key run plan"),
        RekeyReport::SetAside {
            aside,
            mapping,
            unreverted,
        } => {
            console
                .out(&format!("set the run plan aside at {}", aside.display()));
            for line in mapping {
                console.out(line);
            }
            for line in unreverted_listing(unreverted) {
                console.out(&line);
            }
        }
        RekeyReport::UnreadableSetAside { aside } => {
            console.out(&format!(
                "set the unreadable run plan aside at {}",
                aside.display()
            ));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::rc::Rc;

    use corpus::store::AtomicWrite;
    use corpus::store::ExclusiveCreate;
    use corpus::store::FileRemove;
    use corpus::StoreError;
    use corpus_adapters::FileCorpusStore;
    use tracker::ExternalId;
    use tracker::RemoteIssue;
    use tracker::RemoteTimestamp;
    use tracker::RemoteTracker;
    use tracker::TrackerError;
    use tracker_test_support::Call;
    use tracker_test_support::RecordingTracker;
    use vcs::RepoFacts;
    use vcs::RepositoryProbe;
    use vcs::VcsKind;
    use vcs::WorkingCopyState;
    use vcs::WorkingCopyStateProbe;
    use vcs_adapters::library::InProcessProbe;
    use vcs_test_support::hermetic::Hermetic;
    use work::draft_id::DraftId;
    use work::promotion::PromotionRecord;
    use work::promotion::PromotionStage;
    use work::retirement::Retirement;
    use work::retirement::RetirementRecord;
    use work::sync::RequestFingerprint;
    use work::tracker_key::TrackerKey;
    use work_adapters::creation::CreationStore;
    use work_adapters::rekey::run::Forcing;
    use work_adapters::retirement_records::FileRetirementRecords;
    use work_adapters::retirement_records::RetirementRecords as _;
    use work_adapters::sync::pending_push;

    use super::Console;
    use super::RekeyEnvironment;
    use crate::cli::RekeyArgs;
    use crate::test_support::StubRegistry;
    use crate::tracker_registry::SelectionError;
    use crate::tracker_registry::TrackerRegistry;

    pub const KEYED: &str =
        "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n  key: ACC\n---\n";
    pub const KEYED_LINEAR: &str = "---\nwork:\n  integration: linear\n  \
         id_pattern: \"{key}-{number:04d}\"\n  key: ACC\n---\n";
    const NUMERIC: &str = "---\nwork:\n  id_pattern: \"{number:04d}\"\n---\n";
    const TRACKER: &str =
        "---\nwork:\n  integration: linear\n  id_pattern: \"{tracker}\"\n---\n";
    const RUN_PLAN: &str = ".accelerator/state/rekey/run-plan.json";

    pub fn item(id: &str, title: &str, extra: &str) -> String {
        format!("---\nid: \"{id}\"\ntitle: \"{title}\"\n{extra}---\n\n# {id}: {title}\n")
    }

    #[derive(Default)]
    pub struct Recorded {
        out: RefCell<Vec<String>>,
        err: RefCell<Vec<String>>,
    }

    impl Console for Recorded {
        fn out(&self, line: &str) {
            self.out.borrow_mut().push(line.to_owned());
        }

        fn err(&self, line: &str) {
            self.err.borrow_mut().push(line.to_owned());
        }
    }

    #[derive(Debug)]
    pub struct Ran {
        pub code: u8,
        pub out: Vec<String>,
        pub err: String,
    }

    #[derive(Debug)]
    struct Halted;

    /// Stops the run, as a killed process would, at its `at`th mutation.
    struct Halting {
        inner: FileCorpusStore,
        count: Rc<Cell<usize>>,
        at: usize,
    }

    impl Halting {
        fn check(&self) {
            let index = self.count.get();
            self.count.set(index + 1);
            if index == self.at {
                std::panic::resume_unwind(Box::new(Halted));
            }
        }
    }

    impl AtomicWrite for Halting {
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
            self.check();
            self.inner.write(path, bytes)
        }
    }

    impl ExclusiveCreate for Halting {
        fn create_new(
            &self,
            path: &Path,
            bytes: &[u8],
        ) -> Result<(), StoreError> {
            self.check();
            self.inner.create_new(path, bytes)
        }
    }

    impl FileRemove for Halting {
        fn remove(&self, path: &Path) -> Result<(), StoreError> {
            self.check();
            self.inner.remove(path)
        }
    }

    struct Unprobeable;

    impl WorkingCopyStateProbe for Unprobeable {
        fn working_copy_state(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<WorkingCopyState, kernel::Error> {
            Err(kernel::Error::Failed("the index is corrupt".to_owned()))
        }
    }

    impl RepositoryProbe for Unprobeable {
        fn facts_at(&self, start: &Path) -> Option<RepoFacts> {
            Some(RepoFacts {
                root: start.to_path_buf(),
                name: "repo".to_owned(),
                kind: VcsKind::Git,
                revision: None,
            })
        }

        fn user_name_at(&self, _start: &Path) -> Option<String> {
            None
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Action {
        Apply,
        Preview,
        Discard,
    }

    struct Invocation<'a> {
        action: Action,
        forcing: Forcing,
        repository: &'a dyn RepositoryProbe,
        store_at: &'a dyn Fn(&Path) -> Box<dyn CreationStore>,
        registry: &'a dyn TrackerRegistry,
        per_item_reads: bool,
    }

    fn issue(key: &str) -> (ExternalId, RemoteIssue) {
        (
            ExternalId::new(key.to_owned()),
            RemoteIssue {
                key: ExternalId::new(key.to_owned()),
                updated: RemoteTimestamp::Reported("2026-10-01".to_owned()),
                body: String::new(),
            },
        )
    }

    fn external(key: &str) -> ExternalId {
        ExternalId::new(key.to_owned())
    }

    /// A registry resolving `tracker`, and the tracker itself to read its
    /// calls back from.
    fn resolving(
        tracker: RecordingTracker,
    ) -> (StubRegistry, Rc<RecordingTracker>) {
        let tracker = Rc::new(tracker);
        (StubRegistry(Rc::clone(&tracker)), tracker)
    }

    fn holding(keys: &[&str]) -> (StubRegistry, Rc<RecordingTracker>) {
        resolving(RecordingTracker::holding(
            keys.iter().map(|key| issue(key)).collect(),
        ))
    }

    /// A registry that cannot select a tracker, for the reason `selection`
    /// gives.
    struct Unselectable(fn() -> SelectionError);

    impl TrackerRegistry for Unselectable {
        fn resolve(
            &self,
            _name: &str,
        ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
            Err((self.0)())
        }
    }

    fn unconfigured() -> SelectionError {
        SelectionError::Unconfigured {
            name: "linear".to_owned(),
            source: "no credentials".into(),
        }
    }

    pub struct Repo {
        dir: tempfile::TempDir,
        _env_dir: tempfile::TempDir,
        env: Hermetic,
    }

    impl Repo {
        pub fn with(config: &str, files: &[(&str, &str)]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let env_dir = tempfile::tempdir().unwrap();
            let env = Hermetic::rooted_at(env_dir.path()).unwrap();
            let repo = Self {
                dir,
                _env_dir: env_dir,
                env,
            };
            repo.write(".accelerator/config.md", config);
            for (path, content) in files {
                repo.write(path, content);
            }
            repo.env.git(&["init", "--quiet"], repo.root()).unwrap();
            repo.commit();
            repo
        }

        pub fn root(&self) -> &Path {
            self.dir.path()
        }

        pub fn path(&self, relative: &str) -> PathBuf {
            self.root().join(relative)
        }

        pub fn write(&self, relative: &str, content: &str) {
            let path = self.path(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        pub fn read(&self, relative: &str) -> Option<String> {
            std::fs::read_to_string(self.path(relative)).ok()
        }

        pub fn commit(&self) {
            self.env.git(&["add", "-A"], self.root()).unwrap();
            self.env
                .git(
                    &["commit", "--quiet", "--allow-empty", "-m", "snapshot"],
                    self.root(),
                )
                .unwrap();
        }

        pub fn status(&self) -> String {
            self.env
                .git(&["status", "--porcelain", "-uall"], self.root())
                .unwrap()
        }

        pub fn tree(&self) -> BTreeMap<String, String> {
            let mut tree = BTreeMap::new();
            collect(self.root(), &self.path("meta"), &mut tree);
            tree
        }

        fn ran_with(
            &self,
            action: Action,
            forcing: Forcing,
            repository: &dyn RepositoryProbe,
            store_at: &dyn Fn(&Path) -> Box<dyn CreationStore>,
        ) -> Ran {
            self.invoked(&Invocation {
                action,
                forcing,
                repository,
                store_at,
                registry: &holding(&[]).0,
                per_item_reads: false,
            })
        }

        fn invoked(&self, invocation: &Invocation<'_>) -> Ran {
            let console = Recorded::default();
            let composed = config_adapters::compose(
                self.root(),
                config_adapters::LegacyPolicy::Reject,
            )
            .unwrap();
            let code = super::run(
                self.root(),
                &composed.service,
                &RekeyArgs {
                    preview: invocation.action == Action::Preview,
                    discard: invocation.action == Action::Discard,
                    per_item_reads: invocation.per_item_reads,
                },
                &RekeyEnvironment {
                    forcing: invocation.forcing,
                    repository: invocation.repository,
                    store_at: invocation.store_at,
                    registry: invocation.registry,
                    console: &console,
                },
            );
            Ran {
                code,
                out: console.out.take(),
                err: console.err.take().join("\n"),
            }
        }

        pub fn rekey(&self) -> Ran {
            self.ran_with(
                Action::Apply,
                Forcing::Unforced,
                &InProcessProbe,
                &plain,
            )
        }

        fn preview(&self) -> Ran {
            self.ran_with(
                Action::Preview,
                Forcing::Unforced,
                &InProcessProbe,
                &plain,
            )
        }

        fn rekey_forced(&self, repository: &dyn RepositoryProbe) -> Ran {
            self.ran_with(Action::Apply, Forcing::Forced, repository, &plain)
        }

        fn discard(&self) -> Ran {
            self.ran_with(
                Action::Discard,
                Forcing::Unforced,
                &InProcessProbe,
                &plain,
            )
        }

        fn through(
            &self,
            action: Action,
            registry: &dyn TrackerRegistry,
        ) -> Ran {
            self.invoked(&Invocation {
                action,
                forcing: Forcing::Unforced,
                repository: &InProcessProbe,
                store_at: &plain,
                registry,
                per_item_reads: false,
            })
        }

        /// Kills a `work rekey` at its `at`th write.
        fn halt_at(&self, at: usize) {
            self.halt_through(at, &holding(&[]).0);
        }

        /// Kills a `work rekey` resolving its tracker from `registry` at its
        /// `at`th write.
        fn halt_through(&self, at: usize, registry: &dyn TrackerRegistry) {
            let count = Rc::new(Cell::new(0));
            let store_at = |root: &Path| -> Box<dyn CreationStore> {
                Box::new(Halting {
                    inner: FileCorpusStore::new(root),
                    count: Rc::clone(&count),
                    at,
                })
            };
            let halted =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.invoked(&Invocation {
                        action: Action::Apply,
                        forcing: Forcing::Unforced,
                        repository: &InProcessProbe,
                        store_at: &store_at,
                        registry,
                        per_item_reads: false,
                    })
                }));
            assert!(halted.is_err(), "the run finished before write {at}");
        }
    }

    fn plain(root: &Path) -> Box<dyn CreationStore> {
        Box::new(FileCorpusStore::new(root))
    }

    fn collect(root: &Path, dir: &Path, tree: &mut BTreeMap<String, String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, tree);
            } else if let Ok(content) = std::fs::read_to_string(&path) {
                let relative = path.strip_prefix(root).unwrap();
                tree.insert(relative.display().to_string(), content);
            }
        }
    }

    fn legacy_pair() -> Repo {
        Repo::with(
            KEYED,
            &[
                ("meta/work/0041-first.md", &item("0041", "First", "")),
                ("meta/work/0042-second.md", &item("0042", "Second", "")),
                (
                    "meta/notes/links.md",
                    "See work-item:0041 and [second](../work/0042-second.md).\n",
                ),
            ],
        )
    }

    #[test]
    fn a_legacy_item_is_rerendered_under_a_key() {
        let repo = Repo::with(
            KEYED,
            &[("meta/work/0042-search.md", &item("0042", "Search", ""))],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            ran.out,
            vec!["0042 → ACC-0042\tmeta/work/ACC-0042-search.md"]
        );
        assert_eq!(repo.read("meta/work/0042-search.md"), None);
        assert!(repo
            .read("meta/work/ACC-0042-search.md")
            .unwrap()
            .contains("\nid: \"ACC-0042\"\n"));
    }

    #[test]
    fn a_kept_number_and_a_fresh_number_converge() {
        let repo = Repo::with(
            KEYED_LINEAR,
            &[
                (
                    "meta/work/ACC-0041-first.md",
                    &item("ACC-0041", "First", ""),
                ),
                ("meta/work/0042-second.md", &item("0042", "Second", "")),
                (
                    "meta/work/PP-760-synced.md",
                    &item("PP-760", "Synced", "external_id: \"PP-760\"\n"),
                ),
            ],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            ran.out,
            vec![
                "0042 → ACC-0042\tmeta/work/ACC-0042-second.md",
                "PP-760 → ACC-0043\tmeta/work/ACC-0043-synced.md",
            ]
        );
        assert!(repo
            .read("meta/work/ACC-0043-synced.md")
            .unwrap()
            .contains("\nexternal_id: \"PP-760\"\n"));
    }

    #[test]
    fn a_synced_item_named_in_prose_takes_a_fresh_number_and_the_prose_follows()
    {
        let repo = Repo::with(
            KEYED,
            &[
                (
                    "meta/work/ACC-0041-first.md",
                    &item("ACC-0041", "First", ""),
                ),
                (
                    "meta/work/PP-760-synced.md",
                    &item("PP-760", "Synced", "external_id: \"PP-760\"\n"),
                ),
                ("meta/notes/n.md", "Blocked on PP-760.\n"),
            ],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            repo.read("meta/notes/n.md").as_deref(),
            Some("Blocked on ACC-0042.\n")
        );
    }

    #[test]
    fn a_draft_named_in_prose_is_rewritten_and_a_longer_token_is_left() {
        let repo = Repo::with(
            KEYED,
            &[
                (
                    "meta/work/ACC-0041-first.md",
                    &item("ACC-0041", "First", ""),
                ),
                (
                    "meta/work/drafts/draft-k3x9mq-idea.md",
                    &item("draft-k3x9mq", "Idea", "date: \"2026-01-01\"\n"),
                ),
                (
                    "meta/work/PP-760-synced.md",
                    &item(
                        "PP-760",
                        "Synced",
                        "external_id: \"PP-760\"\ndate: \"2026-02-01\"\n",
                    ),
                ),
                ("meta/notes/n.md", "From draft-k3x9mq, not PP-7601.\n"),
            ],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            repo.read("meta/notes/n.md").as_deref(),
            Some("From ACC-0042, not PP-7601.\n")
        );
        assert!(repo.read("meta/work/ACC-0043-synced.md").is_some());
    }

    #[test]
    fn a_prefixed_id_returns_to_bare_numeric_with_a_clean_alias_list() {
        let repo = Repo::with(
            NUMERIC,
            &[(
                "meta/work/ACC-0042-search.md",
                &item("ACC-0042", "Search", "aliases: [\"0042\"]\n"),
            )],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        let retired = repo.read("meta/work/0042-search.md").unwrap();
        assert!(retired.contains("\nid: \"0042\"\n"), "{retired}");
        assert!(retired.contains("\naliases: [\"ACC-0042\"]\n"), "{retired}");
    }

    #[test]
    fn prose_naming_a_prefixed_id_names_the_bare_id() {
        let repo = Repo::with(
            NUMERIC,
            &[
                (
                    "meta/work/ACC-0042-search.md",
                    &item("ACC-0042", "Search", ""),
                ),
                ("meta/notes/n.md", "Builds on ACC-0042.\n"),
            ],
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            repo.read("meta/notes/n.md").as_deref(),
            Some("Builds on 0042.\n")
        );
    }

    #[test]
    fn colliding_targets_refuse_naming_both_and_write_nothing() {
        for (config, files, named) in [
            (
                KEYED,
                vec![
                    ("meta/work/0042-a.md", item("0042", "A", "")),
                    ("meta/work/ACC-0042-b.md", item("ACC-0042", "B", "")),
                ],
                ["0042-a.md", "ACC-0042-b.md"],
            ),
            (
                KEYED,
                vec![
                    ("meta/work/0042-a.md", item("0042", "A", "")),
                    (
                        "meta/work/ACC-0050-b.md",
                        item("ACC-0050", "B", "aliases: [\"ACC-0042\"]\n"),
                    ),
                ],
                ["0042-a.md", "ACC-0050-b.md"],
            ),
            (
                NUMERIC,
                vec![
                    ("meta/work/ACC-0042-a.md", item("ACC-0042", "A", "")),
                    ("meta/work/XYZ-0042-b.md", item("XYZ-0042", "B", "")),
                ],
                ["ACC-0042-a.md", "XYZ-0042-b.md"],
            ),
        ] {
            let files: Vec<(&str, &str)> = files
                .iter()
                .map(|(path, content)| (*path, content.as_str()))
                .collect();
            let repo = Repo::with(config, &files);
            let before = repo.tree();

            let ran = repo.rekey();

            assert_eq!(ran.code, 4, "{ran:?}");
            assert!(
                ran.err.starts_with("Error: E_REKEY_ID_COLLISION"),
                "{ran:?}"
            );
            for name in named {
                assert!(ran.err.contains(name), "{name}: {ran:?}");
            }
            assert_eq!(repo.tree(), before);
            assert_eq!(repo.read(RUN_PLAN), None);
        }
    }

    #[test]
    fn a_draft_with_a_created_pending_push_refuses_naming_it() {
        let repo = Repo::with(
            KEYED_LINEAR,
            &[(
                "meta/work/drafts/draft-k3x9mq-idea.md",
                &item("draft-k3x9mq", "Idea", ""),
            )],
        );
        let integrations = repo.path(".accelerator/state/integrations");
        pending_push::prepare_dir(&integrations, "linear").unwrap();
        repo.write(
            ".accelerator/state/integrations/linear/pending-push/draft-k3x9mq.json",
            &pending_push::render_record(&PromotionRecord {
                draft_id: DraftId::parse("draft-k3x9mq").unwrap(),
                request: RequestFingerprint {
                    title: "Idea".to_owned(),
                    digest: "digest".to_owned(),
                    attempted_at: 1,
                    failure: None,
                },
                content_digest: "digest".to_owned(),
                stage: PromotionStage::Created {
                    key: TrackerKey::parse("ENG-42").unwrap(),
                    created_remote_hash: None,
                },
            }),
        );

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err.starts_with("Error: E_REKEY_PENDING_PUSH"),
            "{ran:?}"
        );
        assert!(ran.err.contains("draft-k3x9mq-idea.md"), "{ran:?}");
    }

    #[test]
    fn an_unfinished_sync_retirement_refuses_naming_it_and_suggests_work_sync()
    {
        let repo = legacy_pair();
        let store = FileCorpusStore::new(repo.root());
        FileRetirementRecords::new(&repo.path(".accelerator/state"), &store)
            .save(&RetirementRecord::of(&Retirement {
                old_id: "draft-k7mq3x",
                new_id: "ENG-42",
                new_external_id: Some("ENG-42"),
            }))
            .unwrap();

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.starts_with("Error: E_REKEY_SYNC_UNFINISHED"));
        assert!(ran.err.contains("draft-k7mq3x"), "{ran:?}");
        assert!(ran.err.contains("Run `accelerator work sync`"), "{ran:?}");
    }

    #[test]
    fn a_dirty_working_copy_refuses_naming_the_paths_and_accelerator_force() {
        let repo = legacy_pair();
        repo.write("meta/notes/links.md", "Edited.\n");
        let before = repo.tree();

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.starts_with("Error: E_REKEY_DIRTY_WORKING_COPY"));
        assert!(ran.err.contains("  meta/notes/links.md"), "{ran:?}");
        assert!(ran.err.contains("ACCELERATOR_FORCE=1"), "{ran:?}");
        assert_eq!(repo.tree(), before);
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn a_failed_probe_refuses_and_forcing_applies() {
        let repo = legacy_pair();

        let refused = repo.ran_with(
            Action::Apply,
            Forcing::Unforced,
            &Unprobeable,
            &plain,
        );
        assert_eq!(refused.code, 4, "{refused:?}");
        assert!(
            refused
                .err
                .starts_with("Error: E_REKEY_WORKING_COPY_UNPROBEABLE"),
            "{refused:?}"
        );
        assert!(refused.err.contains("the index is corrupt"), "{refused:?}");

        let forced = repo.rekey_forced(&Unprobeable);
        assert_eq!(forced.code, 0, "{forced:?}");
        assert!(repo.read("meta/work/ACC-0041-first.md").is_some());
    }

    #[test]
    fn forcing_applies_over_a_dirty_working_copy_and_says_what_it_skipped() {
        let repo = legacy_pair();
        repo.write("meta/notes/links.md", "Edited: work-item:0041.\n");

        let ran = repo.rekey_forced(&InProcessProbe);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.err.contains("ACCELERATOR_FORCE is set"), "{ran:?}");
        assert!(ran.err.contains("working-copy check"), "{ran:?}");
        assert_eq!(
            repo.read("meta/notes/links.md").as_deref(),
            Some("Edited: work-item:ACC-0041.\n")
        );
    }

    #[test]
    fn the_run_plan_does_not_make_the_working_copy_dirty() {
        let repo = legacy_pair();
        repo.halt_at(5);
        assert!(repo.read(RUN_PLAN).is_some());

        let status = repo.status();

        assert!(!status.contains(".accelerator"), "{status}");
    }

    #[test]
    fn an_interrupted_run_finishes_on_the_next_run() {
        let repo = legacy_pair();
        let expected = {
            let uninterrupted = legacy_pair();
            assert_eq!(uninterrupted.rekey().code, 0);
            uninterrupted.tree()
        };
        repo.halt_at(5);
        assert!(repo.read("meta/work/0041-first.md").is_some());
        assert!(repo.read("meta/work/ACC-0041-first.md").is_some());

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(repo.tree(), expected);
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn an_interrupted_run_refuses_on_an_edit_outside_its_write_set() {
        let repo = legacy_pair();
        repo.halt_at(5);
        repo.write("meta/notes/elsewhere.md", "Unrelated.\n");

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.starts_with("Error: E_REKEY_DIRTY_WORKING_COPY"));
        assert!(ran.err.contains("  meta/notes/elsewhere.md"), "{ran:?}");
        assert!(!ran.err.contains("  meta/notes/links.md"), "{ran:?}");
    }

    #[test]
    fn a_malformed_run_plan_refuses_naming_the_file_and_the_discard_remedy() {
        let repo = legacy_pair();
        repo.write(".accelerator/state/rekey/.gitignore", "*\n");
        repo.write(RUN_PLAN, "{\"schema\": 1,");

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.starts_with("Error: E_REKEY_PLAN_UNREADABLE"));
        assert!(ran.err.contains("run-plan.json"), "{ran:?}");
        assert!(ran.err.contains("work rekey --discard"), "{ran:?}");
    }

    #[test]
    fn discard_refuses_an_in_progress_plan_with_its_write_set_changed_and_says_run_work_rekey(
    ) {
        let repo = legacy_pair();
        repo.halt_at(5);

        let ran = repo.discard();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err.starts_with("Error: E_REKEY_NOT_REVERTED"),
            "{ran:?}"
        );
        assert!(
            ran.err
                .contains("Run `accelerator work rekey` to finish it"),
            "{ran:?}"
        );
        assert!(repo.read(RUN_PLAN).is_some());
    }

    fn preview_sync(repo: &Repo) -> std::process::ExitCode {
        let composed = config_adapters::compose(
            repo.root(),
            config_adapters::LegacyPolicy::Reject,
        )
        .unwrap();
        crate::sync::run_sync(
            repo.root(),
            &composed.service,
            &crate::cli::SyncArgs {
                push_only: false,
                pull_only: false,
                preview: true,
                resolutions: Vec::new(),
                per_item_reads: false,
                max_pulls: None,
                max_pushes: None,
                allow_unbounded: false,
                no_promote: false,
                targets: Vec::new(),
            },
            &crate::test_support::StubRegistry(Rc::new(
                tracker_test_support::RecordingTracker::holding(Vec::new()),
            )),
            &crate::finaliser::NoFinaliser,
        )
    }

    #[test]
    fn discard_after_a_vcs_revert_lets_sync_run_again() {
        let repo = Repo::with(
            KEYED_LINEAR,
            &[
                ("meta/work/0041-first.md", &item("0041", "First", "")),
                ("meta/work/0042-second.md", &item("0042", "Second", "")),
            ],
        );
        repo.halt_at(4);
        assert_eq!(
            preview_sync(&repo),
            std::process::ExitCode::from(crate::exit_codes::UNRESOLVED)
        );
        repo.env
            .git(&["checkout", "--quiet", "--", "."], repo.root())
            .unwrap();
        repo.env
            .git(&["clean", "--quiet", "-fd", "meta"], repo.root())
            .unwrap();

        let ran = repo.discard();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(repo.read(RUN_PLAN), None);
        assert_eq!(preview_sync(&repo), std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn discard_with_no_plan_exits_0_through_the_cli() {
        let repo = legacy_pair();

        let ran = repo.discard();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
    }

    #[test]
    fn rekey_in_a_repository_without_a_work_directory_creates_it_and_rekeys_to_nothing(
    ) {
        let repo = Repo::with(KEYED, &[]);

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert!(repo.path("meta/work").is_dir());
    }

    #[test]
    fn a_multi_project_corpus_rekeys_to_nothing() {
        let repo = Repo::with(
            KEYED,
            &[
                ("meta/work/ACC-0001-a.md", &item("ACC-0001", "A", "")),
                ("meta/work/XYZ-0001-b.md", &item("XYZ-0001", "B", "")),
                ("meta/work/AB1-0001-c.md", &item("AB1-0001", "C", "")),
            ],
        );
        let before = repo.tree();

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_eq!(repo.tree(), before);
    }

    #[test]
    fn a_rekeyed_corpus_rekeys_to_nothing() {
        let repo = legacy_pair();
        assert_eq!(repo.rekey().code, 0);
        repo.commit();
        let after = repo.tree();

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_eq!(repo.tree(), after);
    }

    fn previewed_lines(ran: &Ran) -> Vec<String> {
        ran.out
            .iter()
            .map(|line| line.split('\t').next().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn preview_prints_one_line_per_changing_item_and_changes_nothing_under_meta(
    ) {
        let repo = Repo::with(
            KEYED,
            &[
                ("meta/work/ACC-0040-zero.md", &item("ACC-0040", "Zero", "")),
                ("meta/work/0041-first.md", &item("0041", "First", "")),
                (
                    "meta/work/drafts/draft-k3x9mq-idea.md",
                    &item("draft-k3x9mq", "Idea", ""),
                ),
            ],
        );
        let before = repo.tree();

        let ran = repo.preview();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(ran.out, vec!["0041 → ACC-0041", "draft-k3x9mq → ACC-0042"]);
        assert_eq!(repo.tree(), before);
        assert!(repo.read(RUN_PLAN).unwrap().contains("\"proposed\""));
    }

    #[test]
    fn preview_on_a_dirty_working_copy_still_records_a_proposed_plan_and_warns()
    {
        let repo = legacy_pair();
        repo.write("meta/notes/links.md", "Edited.\n");

        let ran = repo.preview();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(ran.out, vec!["0041 → ACC-0041", "0042 → ACC-0042"]);
        assert!(ran.err.contains("  meta/notes/links.md"), "{ran:?}");
        assert!(ran.err.contains("will refuse"), "{ran:?}");
        assert!(repo.read(RUN_PLAN).unwrap().contains("\"proposed\""));
    }

    #[test]
    fn preview_replaces_a_proposed_plan_with_the_new_mapping() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        repo.write("meta/work/0043-third.md", &item("0043", "Third", ""));
        repo.commit();

        let ran = repo.preview();
        let applied = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.contains(&"0043 → ACC-0043".to_owned()), "{ran:?}");
        assert_eq!(applied.code, 0, "{applied:?}");
        assert!(repo.read("meta/work/ACC-0043-third.md").is_some());
    }

    #[test]
    fn preview_of_nothing_records_no_plan() {
        let repo = Repo::with(
            KEYED,
            &[("meta/work/ACC-0001-a.md", &item("ACC-0001", "A", ""))],
        );

        let ran = repo.preview();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn preview_applies_every_refusal_except_the_dirty_tree() {
        let colliding = Repo::with(
            KEYED,
            &[
                ("meta/work/0042-a.md", &item("0042", "A", "")),
                ("meta/work/ACC-0042-b.md", &item("ACC-0042", "B", "")),
            ],
        );
        let unfinished = legacy_pair();
        let store = FileCorpusStore::new(unfinished.root());
        FileRetirementRecords::new(
            &unfinished.path(".accelerator/state"),
            &store,
        )
        .save(&RetirementRecord::of(&Retirement {
            old_id: "draft-k7mq3x",
            new_id: "ENG-42",
            new_external_id: Some("ENG-42"),
        }))
        .unwrap();

        for (repo, code) in [
            (&colliding, "E_REKEY_ID_COLLISION"),
            (&unfinished, "E_REKEY_SYNC_UNFINISHED"),
        ] {
            let ran = repo.preview();

            assert_eq!(ran.code, 4, "{ran:?}");
            assert!(ran.err.starts_with(&format!("Error: {code}")), "{ran:?}");
            assert!(ran.out.is_empty(), "{ran:?}");
            assert_eq!(repo.read(RUN_PLAN), None);
        }
    }

    #[test]
    fn apply_executes_a_proposed_plan_as_previewed() {
        let repo = legacy_pair();
        let previewed = repo.preview();

        let applied = repo.rekey();

        assert_eq!(applied.code, 0, "{applied:?}");
        assert_eq!(previewed_lines(&applied), previewed.out);
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn apply_executes_the_stored_mapping_after_a_committed_date_change() {
        let draft = |date: &str| {
            item("draft-k3x9mq", "Idea", &format!("date: \"{date}\"\n"))
        };
        let repo = Repo::with(
            KEYED,
            &[
                (
                    "meta/work/ACC-0041-first.md",
                    &item("ACC-0041", "First", ""),
                ),
                (
                    "meta/work/drafts/draft-k3x9mq-idea.md",
                    &draft("2026-01-01"),
                ),
                (
                    "meta/work/PP-760-synced.md",
                    &item(
                        "PP-760",
                        "Synced",
                        "external_id: \"PP-760\"\ndate: \"2026-02-01\"\n",
                    ),
                ),
            ],
        );
        let previewed = repo.preview();
        assert_eq!(
            previewed.out,
            vec!["PP-760 → ACC-0043", "draft-k3x9mq → ACC-0042"]
        );
        repo.write(
            "meta/work/drafts/draft-k3x9mq-idea.md",
            &draft("2026-03-01"),
        );
        repo.commit();

        let applied = repo.rekey();

        assert_eq!(applied.code, 0, "{applied:?}");
        let mut lines = previewed_lines(&applied);
        lines.sort();
        assert_eq!(lines, previewed.out);
    }

    fn assert_stale(ran: &Ran, named: &str) {
        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err.starts_with("Error: E_REKEY_PREVIEW_STALE"),
            "{ran:?}"
        );
        assert!(ran.err.contains(named), "{ran:?}");
        assert!(
            ran.err
                .contains("Re-run `accelerator work rekey --preview`"),
            "{ran:?}"
        );
    }

    #[test]
    fn apply_refuses_a_stale_preview_naming_the_changed_items_and_says_re_preview(
    ) {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        repo.write(
            "meta/work/0042-second.md",
            &item("0042", "Second", "aliases: [\"0007\"]\n"),
        );
        repo.commit();
        let before = repo.tree();

        let ran = repo.rekey();

        assert_stale(&ran, "0042-second.md");
        assert_eq!(repo.tree(), before);
    }

    #[test]
    fn apply_refuses_when_an_item_holding_a_previewed_target_was_added() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        repo.write(
            "meta/work/ACC-0041-other.md",
            &item("ACC-0041", "Other", ""),
        );
        repo.commit();
        let before = repo.tree();

        let ran = repo.rekey();

        assert_stale(&ran, "ACC-0041-other.md");
        assert_eq!(repo.tree(), before);
    }

    #[test]
    fn apply_refuses_when_the_scheme_changed_since_preview() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        repo.write(
            ".accelerator/config.md",
            "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n  key: XYZ\n---\n",
        );
        repo.commit();

        let ran = repo.rekey();

        assert_stale(&ran, "`work.id_pattern` or `work.key` changed");
    }

    #[test]
    fn apply_refuses_a_non_item_file_at_a_previewed_target_before_any_write() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        repo.write("meta/work/ACC-0041-first.md", "Not a work item.\n");
        repo.commit();
        let before = repo.tree();

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err.starts_with("Error: E_REKEY_TARGET_EXISTS"),
            "{ran:?}"
        );
        assert_eq!(repo.tree(), before);
    }

    #[test]
    fn apply_on_a_dirty_working_copy_leaves_the_proposed_plan_unchanged() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        let proposed = repo.read(RUN_PLAN);
        repo.write("meta/notes/links.md", "Edited.\n");

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.starts_with("Error: E_REKEY_DIRTY_WORKING_COPY"));
        assert_eq!(repo.read(RUN_PLAN), proposed);
    }

    #[test]
    fn an_interrupted_apply_of_a_proposed_plan_resumes_without_a_fingerprint_check(
    ) {
        let repo = legacy_pair();
        let expected = {
            let uninterrupted = legacy_pair();
            assert_eq!(uninterrupted.rekey().code, 0);
            uninterrupted.tree()
        };
        assert_eq!(repo.preview().code, 0);
        repo.halt_at(5);
        assert!(repo.read(RUN_PLAN).unwrap().contains("\"in_progress\""));

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(repo.tree(), expected);
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn discard_removes_a_proposed_plan_and_changes_nothing_under_meta() {
        let repo = legacy_pair();
        assert_eq!(repo.preview().code, 0);
        let before = repo.tree();

        let ran = repo.discard();

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(repo.read(RUN_PLAN), None);
        assert_eq!(repo.tree(), before);
    }

    #[test]
    fn preview_refuses_while_a_plan_is_in_progress_printing_its_remaining_entries_and_says_run_work_rekey(
    ) {
        let repo = legacy_pair();
        repo.halt_at(5);
        let in_progress = repo.read(RUN_PLAN);

        let ran = repo.preview();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(ran.err.contains("Error: E_REKEY_IN_PROGRESS"), "{ran:?}");
        assert!(
            ran.err
                .contains("Run `accelerator work rekey` to finish it"),
            "{ran:?}"
        );
        assert!(ran.out.contains(&"0042 → ACC-0042".to_owned()), "{ran:?}");
        assert_eq!(repo.read(RUN_PLAN), in_progress);
    }

    #[test]
    fn a_previewed_plan_does_not_make_apply_see_a_dirty_working_copy() {
        let repo = legacy_pair();
        assert!(!repo.path(".accelerator/state/rekey").exists());
        assert_eq!(repo.preview().code, 0);
        assert!(!repo.status().contains(".accelerator"), "{}", repo.status());

        let ran = repo.rekey();

        assert_eq!(ran.code, 0, "{ran:?}");
    }

    #[test]
    fn a_forced_preview_notes_that_apply_will_skip_the_working_copy_check() {
        let repo = legacy_pair();

        let ran = repo.ran_with(
            Action::Preview,
            Forcing::Forced,
            &InProcessProbe,
            &plain,
        );

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.err.contains("ACCELERATOR_FORCE is set"), "{ran:?}");
        assert!(
            ran.err
                .contains("an apply will skip the working-copy check"),
            "{ran:?}"
        );
    }

    #[test]
    fn a_refused_run_leaves_the_plan_as_it_found_it() {
        type Change = (&'static str, fn(&Repo));
        let changes: [Change; 4] = [
            ("stale preview", |repo| {
                repo.write(
                    "meta/work/0042-second.md",
                    &item("0042", "Second", "aliases: [\"0007\"]\n"),
                );
                repo.commit();
            }),
            ("dirty working copy", |repo| {
                repo.write("meta/notes/links.md", "Edited.\n");
            }),
            ("target exists", |repo| {
                repo.write("meta/work/ACC-0041-first.md", "Not an item.\n");
                repo.commit();
            }),
            ("unreadable sync record", |repo| {
                repo.write(
                    ".accelerator/state/retirement-records/0041.json",
                    "{",
                );
            }),
        ];
        for (name, change) in changes {
            let repo = legacy_pair();
            assert_eq!(repo.preview().code, 0, "{name}");
            let proposed = repo.read(RUN_PLAN);
            change(&repo);

            let ran = repo.rekey();

            assert_eq!(ran.code, 4, "{name}: {ran:?}");
            assert_eq!(repo.read(RUN_PLAN), proposed, "{name}");
        }
    }

    fn linked(id: &str, title: &str, key: &str) -> String {
        item(id, title, &format!("external_id: \"{key}\"\n"))
    }

    fn tracker_legacy() -> Repo {
        Repo::with(
            TRACKER,
            &[
                (
                    "meta/work/0042-search.md",
                    &linked("0042", "Search", "PP-760"),
                ),
                (
                    "meta/notes/n.md",
                    "See work-item:0042, [search](../work/0042-search.md) and \
                     ticket 0042.\n",
                ),
            ],
        )
    }

    fn tracker_pair() -> Repo {
        Repo::with(
            TRACKER,
            &[
                ("meta/work/0041-first.md", &linked("0041", "First", "PP-41")),
                ("meta/work/0042-second.md", &linked("0042", "Second", "PP-42")),
                (
                    "meta/notes/links.md",
                    "See work-item:0041 and [second](../work/0042-second.md).\n",
                ),
            ],
        )
    }

    fn assert_unchanged(repo: &Repo, before: &BTreeMap<String, String>) {
        assert_eq!(&repo.tree(), before);
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn a_legacy_item_is_renamed_to_its_tracker_key_and_numeric_prose_is_left() {
        let repo = tracker_legacy();
        let (registry, _) = holding(&["PP-760"]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(ran.out, vec!["0042 → PP-760\tmeta/work/PP-760-search.md"]);
        assert_eq!(repo.read("meta/work/0042-search.md"), None);
        assert_eq!(
            repo.read("meta/notes/n.md").as_deref(),
            Some(
                "See work-item:PP-760, [search](../work/PP-760-search.md) \
                 and ticket 0042.\n"
            )
        );
    }

    #[test]
    fn a_legacy_item_follows_a_moved_key() {
        let repo = tracker_legacy();
        let (registry, _) = resolving(
            RecordingTracker::holding(vec![issue("PP-760")])
                .moving(&external("PP-760"), &external("ENG-42")),
        );

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(ran.out, vec!["0042 → ENG-42\tmeta/work/ENG-42-search.md"]);
        let retired = repo.read("meta/work/ENG-42-search.md").unwrap();
        assert!(retired.contains("\nexternal_id: \"ENG-42\"\n"), "{retired}");
    }

    #[test]
    fn a_synced_item_follows_a_moved_key() {
        let repo = Repo::with(
            TRACKER,
            &[(
                "meta/work/PP-760-search.md",
                &linked("PP-760", "Search", "PP-760"),
            )],
        );
        let (registry, _) = resolving(
            RecordingTracker::holding(vec![issue("PP-760")])
                .moving(&external("PP-760"), &external("ENG-42")),
        );

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(
            ran.out,
            vec!["PP-760 → ENG-42\tmeta/work/ENG-42-search.md"]
        );
        let retired = repo.read("meta/work/ENG-42-search.md").unwrap();
        assert!(retired.contains("\nexternal_id: \"ENG-42\"\n"), "{retired}");
        assert!(retired.contains("\naliases: [\"PP-760\"]\n"), "{retired}");
    }

    #[test]
    fn an_unmoved_synced_item_is_unchanged() {
        let repo = Repo::with(
            TRACKER,
            &[(
                "meta/work/PP-760-search.md",
                &linked("PP-760", "Search", "PP-760"),
            )],
        );
        let before = repo.tree();
        let (registry, _) = holding(&["PP-760"]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_unchanged(&repo, &before);
    }

    #[test]
    fn an_unsynced_legacy_item_becomes_a_draft() {
        let repo = Repo::with(
            TRACKER,
            &[("meta/work/0042-search.md", &item("0042", "Search", ""))],
        );
        let (registry, tracker) = holding(&[]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        let [line] = ran.out.as_slice() else {
            panic!("not one line: {ran:?}");
        };
        let (mapping, path) = line.split_once('\t').unwrap();
        let draft = mapping.strip_prefix("0042 → ").unwrap();
        assert!(DraftId::parse(draft).is_some(), "{line}");
        assert_eq!(path, format!("meta/work/drafts/{draft}-search.md"));
        assert!(repo.read(path).is_some());
        assert_eq!(repo.read("meta/work/0042-search.md"), None);
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
    }

    #[test]
    fn an_existing_draft_is_unchanged_and_no_issue_is_created() {
        let repo = Repo::with(
            TRACKER,
            &[(
                "meta/work/drafts/draft-k3x9mq-idea.md",
                &item("draft-k3x9mq", "Idea", ""),
            )],
        );
        let before = repo.tree();
        let (registry, tracker) = holding(&[]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_unchanged(&repo, &before);
        assert!(
            !tracker
                .calls()
                .iter()
                .any(|call| matches!(call, Call::Create { .. })),
            "{:?}",
            tracker.calls()
        );
    }

    #[test]
    fn a_draft_with_an_attempted_marker_does_not_block_others() {
        let repo = Repo::with(
            TRACKER,
            &[
                (
                    "meta/work/drafts/draft-k3x9mq-idea.md",
                    &item("draft-k3x9mq", "Idea", ""),
                ),
                (
                    "meta/work/0042-search.md",
                    &linked("0042", "Search", "PP-760"),
                ),
            ],
        );
        let integrations = repo.path(".accelerator/state/integrations");
        pending_push::prepare_dir(&integrations, "linear").unwrap();
        repo.write(
            ".accelerator/state/integrations/linear/pending-push/draft-k3x9mq.json",
            &pending_push::render_record(&PromotionRecord {
                draft_id: DraftId::parse("draft-k3x9mq").unwrap(),
                request: RequestFingerprint {
                    title: "Idea".to_owned(),
                    digest: "digest".to_owned(),
                    attempted_at: 1,
                    failure: None,
                },
                content_digest: "digest".to_owned(),
                stage: PromotionStage::Attempted,
            }),
        );
        let (registry, _) = holding(&["PP-760"]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(ran.out, vec!["0042 → PP-760\tmeta/work/PP-760-search.md"]);
        assert!(repo.read("meta/work/drafts/draft-k3x9mq-idea.md").is_some());
    }

    fn unreachable_tracker() -> (StubRegistry, Rc<RecordingTracker>) {
        resolving(
            RecordingTracker::holding(vec![issue("PP-760")]).failing_fetch_all(
                TrackerError::Retryable {
                    detail: "connection refused".to_owned(),
                },
            ),
        )
    }

    fn assert_unreachable(ran: &Ran) {
        assert_eq!(ran.code, 70, "{ran:?}");
        assert!(
            ran.err.contains("Error: E_REKEY_TRACKER_UNREACHABLE"),
            "{ran:?}"
        );
    }

    #[test]
    fn an_unreachable_tracker_exits_70_and_writes_nothing() {
        let repo = tracker_legacy();
        let before = repo.tree();

        let ran = repo.through(Action::Apply, &unreachable_tracker().0);

        assert_unreachable(&ran);
        assert!(ran.err.contains("connection refused"), "{ran:?}");
        assert_unchanged(&repo, &before);
    }

    #[test]
    fn a_failed_locate_after_a_bulk_read_exits_70() {
        let repo = tracker_legacy();
        let before = repo.tree();
        let (registry, _) =
            resolving(RecordingTracker::holding(Vec::new()).failing_locate(
                &external("PP-760"),
                TrackerError::Retryable {
                    detail: "timed out".to_owned(),
                },
            ));

        let ran = repo.through(Action::Apply, &registry);

        assert_unreachable(&ran);
        assert_unchanged(&repo, &before);
    }

    #[test]
    fn an_unreachable_tracker_on_preview_records_no_plan() {
        let repo = tracker_legacy();
        let before = repo.tree();

        let ran = repo.through(Action::Preview, &unreachable_tracker().0);

        assert_unreachable(&ran);
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_unchanged(&repo, &before);
    }

    #[test]
    fn a_refusal_exiting_70_leaves_the_plan_as_it_found_it() {
        let repo = tracker_legacy();
        assert_eq!(
            repo.through(Action::Preview, &holding(&["PP-760"]).0).code,
            0
        );
        let proposed = repo.read(RUN_PLAN);
        assert!(proposed.is_some());

        let ran = repo.through(Action::Preview, &unreachable_tracker().0);

        assert_unreachable(&ran);
        assert_eq!(repo.read(RUN_PLAN), proposed);
    }

    fn moved_since_preview() -> (StubRegistry, Rc<RecordingTracker>) {
        resolving(
            RecordingTracker::holding(vec![issue("PP-760")])
                .moving(&external("PP-760"), &external("ENG-42")),
        )
    }

    #[test]
    fn applying_a_proposed_plan_makes_no_tracker_call() {
        let repo = tracker_legacy();
        assert_eq!(
            repo.through(Action::Preview, &holding(&["PP-760"]).0).code,
            0
        );
        let (registry, tracker) = moved_since_preview();

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
        let retired = repo.read("meta/work/PP-760-search.md").unwrap();
        assert!(retired.contains("\nid: \"PP-760\"\n"), "{retired}");
    }

    #[test]
    fn the_confirmed_ids_equal_the_previewed_lines_after_a_later_move() {
        let repo = tracker_legacy();
        let previewed = repo.through(Action::Preview, &holding(&["PP-760"]).0);
        assert_eq!(previewed.out, vec!["0042 → PP-760"]);

        let applied = repo.through(Action::Apply, &moved_since_preview().0);

        assert_eq!(applied.code, 0, "{applied:?}");
        assert_eq!(previewed_lines(&applied), previewed.out);
    }

    #[test]
    fn finishing_an_in_progress_plan_makes_no_tracker_call() {
        let repo = tracker_pair();
        repo.halt_through(5, &holding(&["PP-41", "PP-42"]).0);
        assert!(repo.read(RUN_PLAN).unwrap().contains("\"in_progress\""));
        let (registry, tracker) = holding(&["PP-41", "PP-42"]);

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(tracker.calls().is_empty(), "{:?}", tracker.calls());
        assert!(repo.read("meta/work/PP-41-first.md").is_some());
        assert!(repo.read("meta/work/PP-42-second.md").is_some());
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn finishing_an_in_progress_plan_needs_no_tracker_configuration() {
        let repo = tracker_pair();
        repo.halt_through(5, &holding(&["PP-41", "PP-42"]).0);

        let ran = repo.through(Action::Apply, &Unselectable(unconfigured));

        assert_eq!(ran.code, 0, "{ran:?}");
        assert_eq!(repo.read(RUN_PLAN), None);
    }

    #[test]
    fn a_rekeyed_corpus_under_the_tracker_pattern_rekeys_to_nothing() {
        let repo = tracker_pair();
        let (registry, _) = holding(&["PP-41", "PP-42"]);
        assert_eq!(repo.through(Action::Apply, &registry).code, 0);
        repo.commit();
        let after = repo.tree();

        let ran = repo.through(Action::Apply, &registry);

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.out.is_empty(), "{ran:?}");
        assert_eq!(repo.tree(), after);
    }

    #[test]
    fn colliding_tracker_targets_refuse_and_write_nothing() {
        let repo = Repo::with(
            TRACKER,
            &[
                ("meta/work/0041-a.md", &linked("0041", "A", "PP-760")),
                ("meta/work/0042-b.md", &linked("0042", "B", "PP-760")),
            ],
        );
        let before = repo.tree();

        let ran = repo.through(Action::Apply, &holding(&["PP-760"]).0);

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err.starts_with("Error: E_REKEY_ID_COLLISION"),
            "{ran:?}"
        );
        assert!(ran.err.contains("0041-a.md"), "{ran:?}");
        assert!(ran.err.contains("0042-b.md"), "{ran:?}");
        assert_unchanged(&repo, &before);
    }

    #[test]
    fn an_unconfigured_tracker_exits_with_its_selection_code() {
        let selections: [(fn() -> SelectionError, u8); 4] = [
            (
                || SelectionError::NotAvailable {
                    name: "trello".to_owned(),
                },
                crate::exit_codes::NOT_AVAILABLE,
            ),
            (|| SelectionError::Unset, crate::exit_codes::UNRECOGNISED),
            (
                || SelectionError::Unrecognised {
                    name: "nope".to_owned(),
                },
                crate::exit_codes::UNRECOGNISED,
            ),
            (unconfigured, crate::exit_codes::UNCONFIGURED),
        ];
        for (selection, code) in selections {
            let repo = tracker_legacy();
            let before = repo.tree();

            let ran = repo.through(Action::Apply, &Unselectable(selection));

            assert_eq!(ran.code, code, "{ran:?}");
            assert!(ran.err.contains(&selection().message()), "{ran:?}");
            assert!(
                !ran.err.contains("E_REKEY_TRACKER_UNREACHABLE"),
                "{ran:?}"
            );
            assert_unchanged(&repo, &before);
        }
    }

    #[test]
    fn per_item_reads_under_a_numeric_pattern_is_ignored_with_a_note() {
        let repo = legacy_pair();

        let ran = repo.invoked(&Invocation {
            action: Action::Apply,
            forcing: Forcing::Unforced,
            repository: &InProcessProbe,
            store_at: &plain,
            registry: &holding(&[]).0,
            per_item_reads: true,
        });

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(ran.err.contains("--per-item-reads"), "{ran:?}");
        assert!(ran.err.contains("ignored"), "{ran:?}");
    }

    #[test]
    fn per_item_reads_under_the_tracker_pattern_locates_each_key() {
        let repo = tracker_legacy();
        let (registry, tracker) = holding(&["PP-760"]);

        let ran = repo.invoked(&Invocation {
            action: Action::Apply,
            forcing: Forcing::Unforced,
            repository: &InProcessProbe,
            store_at: &plain,
            registry: &registry,
            per_item_reads: true,
        });

        assert_eq!(ran.code, 0, "{ran:?}");
        assert!(!ran.err.contains("--per-item-reads"), "{ran:?}");
        assert_eq!(
            tracker.calls(),
            vec![Call::Locate {
                id: external("PP-760")
            }]
        );
    }
}
