//! `accelerator work rekey`: converges every work item onto the configured
//! `work.id_pattern`, finishing an interrupted run first, or discards a run
//! a revert has undone.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus::IdOwnership;
use corpus::WorkItemIdScheme;
use vcs::guarded_working_copy::GUARDED_SCOPES;
use vcs::RepositoryProbe;
use work::rekey::entry::RekeyEntry;
use work::rekey::outcome::unreverted_listing;
use work::rekey::outcome::Holder;
use work::rekey::outcome::NotRekeyed;
use work_adapters::creation::CreationStore;
use work_adapters::filesystem::FilesystemWorkItemFiles;
use work_adapters::identity_run::waiting_line;
use work_adapters::identity_run_lock::IdentityRunLock;
use work_adapters::rekey::execute::RekeyCorpus;
use work_adapters::rekey::guards::PendingPushDir;
use work_adapters::rekey::guards::SyncState;
use work_adapters::rekey::run::Forcing;
use work_adapters::rekey::run::RekeyAction;
use work_adapters::rekey::run::RekeyPorts;
use work_adapters::rekey::run::RekeyProgress;
use work_adapters::rekey::run::RekeyReport;
use work_adapters::rekey::run::RekeyRequest;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;
use work_adapters::sync::baseline_store::BaselineStore;

use crate::cli::RekeyArgs;
use crate::exit_codes;
use crate::identity_workspace::CorpusWorkspace;
use crate::identity_workspace::IdentityWorkspace;

pub const E_REKEY_TRACKER_PATTERN_PENDING: &str =
    "E_REKEY_TRACKER_PATTERN_PENDING";

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

const fn skipped_by_force(action: RekeyAction) -> &'static str {
    match action {
        RekeyAction::Apply => {
            "warning: ACCELERATOR_FORCE is set: skipping the working-copy \
             check, so version control may not be able to undo this run"
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

/// Runs `work rekey` and returns its exit code.
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
    let action = if args.discard {
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
    if located.scheme.ownership() == IdOwnership::Tracker {
        console.err(&format!(
            "Error: {E_REKEY_TRACKER_PATTERN_PENDING}: re-keying is not yet \
             supported under `{{tracker}}`. Re-key under a numeric pattern, \
             or wait for a release that supports it."
        ));
        return exit_codes::UNRESOLVED;
    }
    let workspace = match Workspace::open(config, &located) {
        Ok(workspace) => workspace,
        Err(message) => return internal(message),
    };
    let request = RekeyRequest {
        action,
        forcing: environment.forcing,
        scheme: located.scheme.clone(),
    };
    match rekey(&request, &located, &workspace, environment) {
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
        },
    )
}

fn render(report: &RekeyReport, console: &dyn Console) {
    match report {
        RekeyReport::AlreadyConverged => {
            console.err("every work item already matches `work.id_pattern`");
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
            discard: bool,
            forcing: Forcing,
            repository: &dyn RepositoryProbe,
            store_at: &dyn Fn(&Path) -> Box<dyn CreationStore>,
        ) -> Ran {
            let console = Recorded::default();
            let composed = config_adapters::compose(
                self.root(),
                config_adapters::LegacyPolicy::Reject,
            )
            .unwrap();
            let code = super::run(
                self.root(),
                &composed.service,
                &RekeyArgs { discard },
                &RekeyEnvironment {
                    forcing,
                    repository,
                    store_at,
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
            self.ran_with(false, Forcing::Unforced, &InProcessProbe, &plain)
        }

        fn rekey_forced(&self, repository: &dyn RepositoryProbe) -> Ran {
            self.ran_with(false, Forcing::Forced, repository, &plain)
        }

        fn discard(&self) -> Ran {
            self.ran_with(true, Forcing::Unforced, &InProcessProbe, &plain)
        }

        /// Kills a `work rekey` at its `at`th write.
        fn halt_at(&self, at: usize) {
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
                    self.ran_with(
                        false,
                        Forcing::Unforced,
                        &InProcessProbe,
                        &store_at,
                    )
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

        let refused =
            repo.ran_with(false, Forcing::Unforced, &Unprobeable, &plain);
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

    #[test]
    fn the_tracker_pattern_is_refused_until_supported() {
        let repo = Repo::with(
            TRACKER,
            &[("meta/work/0042-search.md", &item("0042", "Search", ""))],
        );
        let before = repo.tree();

        let ran = repo.rekey();

        assert_eq!(ran.code, 4, "{ran:?}");
        assert!(
            ran.err
                .starts_with("Error: E_REKEY_TRACKER_PATTERN_PENDING"),
            "{ran:?}"
        );
        assert_eq!(repo.tree(), before);
    }
}
