//! Reads git through `gix` and jj through `jj-lib`, in-process.
//!
//! Three walks, and using the wrong one is the mistake to avoid. The combined
//! `.jj`-or-`.git` marker walk answers `RepoRoot::discover` only. A `.jj`-only
//! walk answers the jj queries, because `DefaultWorkspaceLoaderFactory::create`
//! performs no walk of its own and the combined boundary makes it report absence
//! on a git checkout nested inside a jj workspace, where `jj workspace root`
//! reports a root. `gix::discover` performs the third.
//!
//! `gix_discover::upwards::Options::ceiling_dirs` cannot confine a walk to the
//! boundary: it derives the ceiling height from
//! `strip_prefix(ceiling).components().count()`, discards height 0, and tests
//! `current_height > max_height` before incrementing.
//!
//! Every returned path goes through `canonicalise`/`canonical`. `repo_path()`
//! arrives canonicalised from jj-lib while `workspace_root()` is whatever was
//! passed in, and a linked worktree's `workdir()` is reconstructed from the path
//! git recorded at `git worktree add` time.
//!
//! The cargo-pup import rule resolves a grouped `use a::{b, c}` to an empty
//! module name and rejects it, so every import here is single-item.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::future::Future;
use std::path::absolute;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use etcetera::BaseStrategy as _;
use jj_lib::backend::CommitId;
use jj_lib::config::ConfigGetError;
use jj_lib::config::ConfigLayer;
use jj_lib::config::ConfigSource;
use jj_lib::config::StackedConfig;
use jj_lib::object_id::ObjectId;
use jj_lib::op_store::OpStore as _;
use jj_lib::op_store::OpStoreError;
use jj_lib::op_store::OperationId;
use jj_lib::op_store::RootOperationData;
use jj_lib::protos::local_working_copy::Checkout as CheckoutProto;
use jj_lib::ref_name::WorkspaceName;
use jj_lib::ref_name::WorkspaceNameBuf;
use jj_lib::simple_op_store::SimpleOpStore;
use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::WorkspaceLoadError;
use jj_lib::workspace::WorkspaceLoaderFactory as _;
use prost::Message as _;
use tracing::warn;
use vcs::checkout::DualRoots;
use vcs::checkout::JjRepositoryFacts;
use vcs::checkout::JjWorkspaceRole;
use vcs::checkout::WorktreeFacts;
use vcs::origin_remote::OriginRemote;
use vcs::RepoRoot;
use vcs::UserIdentityProbe;
use vcs::VcsKind;
use vcs::VcsProbe;

use crate::markers::carries_any_marker;
use crate::markers::carries_jj_marker;
use crate::markers::marker_kind;
use crate::markers::walk_up;

/// A repository is here and the pinned library could not answer.
#[derive(Debug)]
pub enum Error {
    Absolutise {
        path: PathBuf,
        source: std::io::Error,
    },
    Canonicalise {
        path: PathBuf,
        source: std::io::Error,
    },
    Git {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    Jj {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    JjStoreLayout {
        store: PathBuf,
    },
    JjWorkingCopyBackend {
        backend: String,
    },
    JjCheckout {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    JjOpStore {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    JjConfig {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolutise { path, .. } => {
                write!(formatter, "could not absolutise {}", path.display())
            }
            Self::Canonicalise { path, .. } => {
                write!(formatter, "could not canonicalise {}", path.display())
            }
            Self::Git { path, .. } => {
                write!(
                    formatter,
                    "could not read the git repository at {}",
                    path.display()
                )
            }
            Self::Jj { path, .. } => {
                write!(
                    formatter,
                    "could not read the jj workspace at {}",
                    path.display()
                )
            }
            Self::JjStoreLayout { store } => {
                write!(
                    formatter,
                    "{} is not a jj repository store",
                    store.display()
                )
            }
            Self::JjWorkingCopyBackend { backend } => {
                write!(
                    formatter,
                    "unsupported jj working copy backend {backend:?}"
                )
            }
            Self::JjCheckout { path, .. } => {
                write!(
                    formatter,
                    "could not read the jj checkout state at {}",
                    path.display()
                )
            }
            Self::JjOpStore { .. } => {
                write!(formatter, "could not read the jj operation store")
            }
            Self::JjConfig { .. } => {
                write!(formatter, "could not read the jj configuration")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Absolutise { source, .. }
            | Self::Canonicalise { source, .. } => Some(source),
            Self::Git { source, .. }
            | Self::Jj { source, .. }
            | Self::JjCheckout { source, .. }
            | Self::JjOpStore { source }
            | Self::JjConfig { source } => Some(source.as_ref()),
            Self::JjStoreLayout { .. } | Self::JjWorkingCopyBackend { .. } => {
                None
            }
        }
    }
}

/// Reads a repository's root, idiom and revision in-process.
#[derive(Debug, Clone, Copy, Default)]
pub struct InProcessProbe;

impl InProcessProbe {
    /// Whether the repository containing `start` is bare.
    ///
    /// A bare repository carries neither marker, so it is unreachable from the
    /// boundary walk and needs this entry point of its own.
    ///
    /// # Errors
    ///
    /// When a repository is present but cannot be opened or configured.
    pub fn is_bare(&self, start: &Path) -> Result<Option<bool>, Error> {
        let start = absolutise(start)?;
        Ok(discover_git(&start)?.map(|repository| repository.is_bare()))
    }

    /// Whether the checkout containing `start` is a linked worktree, and where
    /// its git directories are.
    ///
    /// # Errors
    ///
    /// When a repository is present but its directories cannot be read.
    pub fn worktree(
        &self,
        start: &Path,
    ) -> Result<Option<WorktreeFacts>, Error> {
        let start = absolutise(start)?;
        let Some(repository) = discover_git(&start)? else {
            return Ok(None);
        };

        let git_dir = canonicalise(repository.git_dir())?;
        // Raw, this carries `../..` for a linked worktree.
        let common_dir = canonicalise(repository.common_dir())?;

        let main = repository.main_repo().map_err(|error| Error::Git {
            path: git_dir.clone(),
            source: Box::new(error),
        })?;
        let main_worktree_root =
            main.workdir().map(canonicalise).transpose()?;

        Ok(Some(WorktreeFacts {
            linked: git_dir != common_dir,
            git_dir,
            common_dir,
            main_worktree_root,
        }))
    }

    /// The superproject's working directory, when `start` is inside a submodule.
    ///
    /// Derived from the `git_dir()` path shape: gix has no API for this
    /// direction, and `main_repo()` on a submodule returns the submodule.
    ///
    /// # Errors
    ///
    /// When a candidate superproject exists but cannot be opened.
    pub fn superproject(&self, start: &Path) -> Result<Option<PathBuf>, Error> {
        let start = absolutise(start)?;
        let Some(repository) = discover_git(&start)? else {
            return Ok(None);
        };

        // Gating on `kind() == Submodule` is wrong in both directions: it
        // admits a linked worktree *of* a submodule, for which git reports no
        // superproject, and rejects a submodule inside a linked worktree, for
        // which git reports one. The dirs comparison is git's own discriminator.
        let git_dir = canonicalise(repository.git_dir())?;
        if git_dir != canonicalise(repository.common_dir())? {
            return Ok(None);
        }

        superproject_of(&git_dir, |candidate| {
            let Some(found) = open_git(candidate)? else {
                return Ok(None);
            };
            found.workdir().map(canonicalise).transpose()
        })
    }

    /// The jj workspace root containing `start`.
    ///
    /// # Errors
    ///
    /// When a `.jj` directory is present but the workspace cannot be loaded.
    pub fn jj_workspace_root(
        &self,
        start: &Path,
    ) -> Result<Option<PathBuf>, Error> {
        let start = absolutise(start)?;
        let Some(root) = walk_up(&start, carries_jj_marker) else {
            return Ok(None);
        };
        let Some(loader) = load_jj_workspace(&root)? else {
            return Ok(None);
        };
        Ok(Some(canonicalise(loader.workspace_root())?))
    }

    /// Whether the jj workspace containing `start` owns its store or shares
    /// another's, and where that repository is.
    ///
    /// # Errors
    ///
    /// When the workspace cannot be loaded, or its store does not resolve to a
    /// jj repository layout.
    pub fn jj_repository(
        &self,
        start: &Path,
    ) -> Result<Option<JjRepositoryFacts>, Error> {
        let start = absolutise(start)?;
        let Some(root) = walk_up(&start, carries_jj_marker) else {
            return Ok(None);
        };
        let Some(loader) = load_jj_workspace(&root)? else {
            return Ok(None);
        };

        let store = canonicalise(loader.repo_path())?;
        let own_store = canonicalise(&root.join(".jj").join("repo"))?;
        let role = if store == own_store {
            JjWorkspaceRole::Main
        } else {
            JjWorkspaceRole::Secondary
        };

        let Some(main_root) = store.parent().and_then(Path::parent) else {
            return Err(Error::JjStoreLayout { store });
        };
        // Without this, a `.jj/repo` pointing at any existing directory yields
        // a real-looking but wrong root two levels up.
        if !main_root.join(".jj").join("repo").is_dir() {
            return Err(Error::JjStoreLayout { store });
        }

        Ok(Some(JjRepositoryFacts {
            role,
            main_root: main_root.to_path_buf(),
        }))
    }

    /// The git repository root and the jj workspace root, resolved
    /// independently.
    pub fn dual_roots(&self, start: &Path) -> DualRoots {
        DualRoots {
            git: git_root(start).map_err(Into::into),
            jj: self.jj_workspace_root(start).map_err(Into::into),
        }
    }

    /// The `origin` remote's configured URL, read from whichever config file
    /// `gix` resolves it from. `Ok(None)` only for a repository that opens
    /// cleanly but carries no `origin` remote.
    ///
    /// Unlike [`Self::worktree`]/[`Self::is_bare`]'s use of [`discover_git`]
    /// (which folds "no repository here at all" to `Ok(None)`, right for a
    /// query that walks from an arbitrary start point), `root` here is
    /// always a repository root the caller already discovered — so `gix`
    /// failing to find one there at all is itself an inconsistency worth
    /// surfacing, not a clean absence. Unlike this crate's
    /// `UserIdentityProbe`-facing `git_user_name`, which folds every
    /// `gix::discover` failure to `None`, every failure here propagates as
    /// `Err`, per [`vcs::origin_remote::OriginRemote`]'s contract.
    ///
    /// # Errors
    ///
    /// When the repository cannot be opened.
    pub fn origin_url(&self, start: &Path) -> Result<Option<String>, Error> {
        let start = absolutise(start)?;
        let repository = gix::discover(&start).map_err(|error| Error::Git {
            path: start.clone(),
            source: Box::new(error),
        })?;
        Ok(repository
            .config_snapshot()
            .string("remote.origin.url")
            .map(|value| value.to_string()))
    }
}

impl From<Error> for kernel::Error {
    fn from(error: Error) -> Self {
        Self::Failed(error.to_string())
    }
}

/// The git working-copy root containing `start`, by gix's own walk so it is not
/// truncated by a `.jj` marker. `Ok(None)` for a bare repository, which has no
/// working copy.
fn git_root(start: &Path) -> Result<Option<PathBuf>, Error> {
    let start = absolutise(start)?;
    let Some(repository) = discover_git(&start)? else {
        return Ok(None);
    };
    repository.workdir().map(canonicalise).transpose()
}

impl vcs::classify::CheckoutProbe for InProcessProbe {
    fn is_bare(&self, start: &Path) -> Result<Option<bool>, kernel::Error> {
        self.is_bare(start).map_err(Into::into)
    }

    fn worktree(
        &self,
        start: &Path,
    ) -> Result<Option<WorktreeFacts>, kernel::Error> {
        self.worktree(start).map_err(Into::into)
    }

    fn jj_repository(
        &self,
        start: &Path,
    ) -> Result<Option<JjRepositoryFacts>, kernel::Error> {
        self.jj_repository(start).map_err(Into::into)
    }

    fn dual_roots(&self, start: &Path) -> DualRoots {
        self.dual_roots(start)
    }
}

impl vcs::mode::ModeProbe for InProcessProbe {
    fn jj_workspace_root(
        &self,
        start: &Path,
    ) -> Result<Option<PathBuf>, kernel::Error> {
        self.jj_workspace_root(start).map_err(Into::into)
    }

    fn dual_roots(&self, start: &Path) -> DualRoots {
        self.dual_roots(start)
    }
}

impl RepoRoot for InProcessProbe {
    fn discover(&self, start: &Path) -> Option<PathBuf> {
        walk_up(start, carries_any_marker).map(|root| canonical(&root))
    }

    fn repository_root(&self, working_copy_root: &Path) -> PathBuf {
        jj_repository_root(working_copy_root)
            .unwrap_or_else(|| canonical(working_copy_root))
    }
}

impl VcsProbe for InProcessProbe {
    fn kind(&self, root: &Path) -> VcsKind {
        marker_kind(root)
    }

    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String> {
        match kind {
            VcsKind::Git => git_revision(root),
            VcsKind::Jj => match jj_revision(root) {
                Ok(revision) => revision,
                Err(error) => {
                    warn!(
                        vcs = "jj",
                        %error, "could not read the working-copy commit id"
                    );
                    None
                }
            },
            VcsKind::None => None,
        }
    }
}

impl UserIdentityProbe for InProcessProbe {
    fn user_name(&self, root: &Path, kind: VcsKind) -> Option<String> {
        match kind {
            VcsKind::Git => git_user_name(root),
            VcsKind::Jj => match jj_user_name() {
                Ok(name) => name,
                Err(error) => {
                    warn!(
                        vcs = "jj",
                        %error, "could not read the configured user name"
                    );
                    None
                }
            },
            VcsKind::None => None,
        }
    }
}

impl OriginRemote for InProcessProbe {
    fn origin_url(&self, root: &Path) -> Result<Option<String>, kernel::Error> {
        self.origin_url(root).map_err(Into::into)
    }
}

/// The repository a jj working copy belongs to.
///
/// The store is always `<repository>/.jj/repo`, so the repository is its
/// grandparent: the main repository for a secondary workspace, the workspace
/// itself otherwise.
fn jj_repository_root(working_copy_root: &Path) -> Option<PathBuf> {
    let loader = match DefaultWorkspaceLoaderFactory.create(working_copy_root) {
        Ok(loader) => loader,
        Err(WorkspaceLoadError::NoWorkspaceHere(_)) => return None,
        Err(error) => {
            warn!(%error, "could not load the jj workspace");
            return None;
        }
    };

    let store = loader.repo_path();
    let Some(repository) = store.parent().and_then(Path::parent) else {
        warn!(
            store = %store.display(),
            "the jj store has no repository root above it"
        );
        return None;
    };
    Some(canonical(repository))
}

/// The full working-copy revision of a git checkout. `None`, unlogged, for a
/// repository with no commits yet.
fn git_revision(root: &Path) -> Option<String> {
    let repository = match gix::discover(root) {
        Ok(repository) => repository,
        Err(error) => {
            warn!(
                vcs = "git",
                %error, "could not open the repository for its revision"
            );
            return None;
        }
    };

    let head = repository.head_commit();
    match head {
        Ok(commit) => Some(commit.id().to_string()),
        Err(error) if is_unborn_head(&error) => None,
        Err(error) => {
            warn!(vcs = "git", %error, "could not read the head commit");
            None
        }
    }
}

/// The configured `user.name` of a git repository, read from whichever config
/// file `gix` resolves it from (system, global, local, worktree — in that
/// precedence). `None`, unlogged, when the key is unset.
fn git_user_name(root: &Path) -> Option<String> {
    let repository = match gix::discover(root) {
        Ok(repository) => repository,
        Err(error) => {
            warn!(
                vcs = "git",
                %error, "could not open the repository for its configured \
                 user name"
            );
            return None;
        }
    };

    repository
        .config_snapshot()
        .string("user.name")
        .map(|value| value.to_string())
}

/// The full working-copy revision of a jj workspace, read without constructing
/// any settings and without writing anything.
///
/// The workspace's `checkout` file records the operation the working copy is at
/// and the workspace's own name; that operation's view maps every workspace name
/// to its working-copy commit. `SimpleOpStore::load` takes a path only, which is
/// what keeps the route settings-free — the higher-level repository loader needs
/// a settings value, and the working-copy loader it reaches writes to the store.
///
/// Indexing by name is load-bearing: a repository with several workspaces holds
/// a different commit per workspace, so taking the sole view entry would answer
/// for the wrong one.
///
/// Reports the commit as of the last recorded operation, and does not snapshot.
/// Asking the `jj` binary does snapshot, so it reports — and writes — a new
/// commit when files changed since the last jj command.
fn jj_revision(root: &Path) -> Result<Option<String>, Error> {
    let Some(loader) = load_jj_workspace(root)? else {
        return Ok(None);
    };

    // The paths below belong to the local backend, so an unfamiliar one has to
    // fail by name rather than as a missing file.
    let backend =
        loader.get_working_copy_type().map_err(|error| Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        })?;
    if backend != LOCAL_WORKING_COPY {
        return Err(Error::JjWorkingCopyBackend { backend });
    }

    let state = loader.workspace_root().join(".jj").join("working_copy");
    let checkout = read_checkout(&state)?;

    // Only the root operation consults this, and the checkout file gives a
    // concrete id, so it is never read.
    let root_data = RootOperationData {
        root_commit_id: CommitId::from_bytes(&[0; 20]),
    };
    let store =
        SimpleOpStore::load(&loader.repo_path().join("op_store"), root_data);

    let operation = block_on_jj(store.read_operation(&checkout.operation))?;
    let view = block_on_jj(store.read_view(&operation.view_id))?;

    Ok(view
        .wc_commit_ids
        .get(&checkout.workspace)
        .map(ObjectId::hex))
}

const LOCAL_WORKING_COPY: &str = "local";

struct Checkout {
    operation: OperationId,
    workspace: WorkspaceNameBuf,
}

/// Decodes `<state>/checkout` into the operation and workspace name it records.
///
/// The empty-name fallback mirrors jj's own, for working copies written before
/// the field existed.
fn read_checkout(state: &Path) -> Result<Checkout, Error> {
    let path = state.join("checkout");
    let bytes = fs::read(&path).map_err(|source| Error::JjCheckout {
        path: path.clone(),
        source: Box::new(source),
    })?;
    let proto =
        CheckoutProto::decode(&*bytes).map_err(|source| Error::JjCheckout {
            path,
            source: Box::new(source),
        })?;

    let workspace = if proto.workspace_name.is_empty() {
        WorkspaceName::DEFAULT.to_owned()
    } else {
        proto.workspace_name.into()
    };
    Ok(Checkout {
        operation: OperationId::new(proto.operation_id),
        workspace,
    })
}

/// The configured `user.name` for the ambient jj configuration, read without
/// constructing `UserSettings` — which additionally requires
/// `operation.hostname`, `signing.behavior`, and other fields unrelated to
/// identity to resolve.
///
/// Replicates jj-cli's own config-stack precedence for the layers that can
/// affect `user.name`: system config, user config (either `$JJ_CONFIG`'s
/// paths, or the legacy `~/.jjconfig.toml` plus the platform
/// `config.toml`/`conf.d`), and the `JJ_USER` override — the same layers
/// `jj config get user.name` would consult for this key. The built-in default
/// layers (colours, merge tools, revsets, ...) never set `user.name` so are
/// skipped, as is the `EnvBase` layer (hostname/username/editor). Repo and
/// workspace config are skipped too: jj's repo-config indirection — a config
/// ID stored in the repo, resolving to content in the user's own config
/// directory — is disproportionate machinery to replicate for a rarely-used
/// per-repository override.
fn jj_user_name() -> Result<Option<String>, Error> {
    let mut config = StackedConfig::empty();
    let env_jj_config = std::env::var_os("JJ_CONFIG");

    if env_jj_config.is_none() {
        for path in jj_system_config_paths() {
            load_jj_config_path(&mut config, ConfigSource::System, &path)?;
        }
    }
    for path in jj_user_config_paths(env_jj_config.as_deref()) {
        load_jj_config_path(&mut config, ConfigSource::User, &path)?;
    }
    if let Ok(user) = std::env::var("JJ_USER") {
        let mut layer = ConfigLayer::empty(ConfigSource::EnvOverrides);
        layer.set_value("user.name", user).map_err(|error| {
            Error::JjConfig {
                source: Box::new(error),
            }
        })?;
        config.add_layer(layer);
    }

    match config.get::<String>("user.name") {
        Ok(name) => Ok(Some(name)),
        Err(ConfigGetError::NotFound { .. }) => Ok(None),
        Err(error) => Err(Error::JjConfig {
            source: Box::new(error),
        }),
    }
}

/// Loads `path` into `config` at `source`, as a directory of `*.toml` layers
/// or a single file, matching jj-cli's own dispatch. A path that is neither
/// (does not exist) is silently skipped, since the candidate lists below
/// carry paths that may not exist yet.
fn load_jj_config_path(
    config: &mut StackedConfig,
    source: ConfigSource,
    path: &Path,
) -> Result<(), Error> {
    let outcome = if path.is_dir() {
        config.load_dir(source, path)
    } else if path.is_file() {
        config.load_file(source, path)
    } else {
        return Ok(());
    };
    outcome.map_err(|error| Error::JjConfig {
        source: Box::new(error),
    })
}

/// `/etc/jj/config.toml` and `/etc/jj/conf.d`, jj-cli's system-config
/// candidates on Unix. Windows has none.
fn jj_system_config_paths() -> Vec<PathBuf> {
    if cfg!(unix) {
        vec![
            PathBuf::from("/etc/jj/config.toml"),
            PathBuf::from("/etc/jj/conf.d"),
        ]
    } else {
        Vec::new()
    }
}

/// jj-cli's user-config candidates: `$JJ_CONFIG`'s paths when set (used
/// exclusively), else the legacy `~/.jjconfig.toml` (only when it exists, or
/// when the platform config directory could not be resolved at all), the
/// platform `config.toml` (always a candidate, whether or not it exists yet),
/// and the platform `conf.d` (only when it exists).
fn jj_user_config_paths(env_jj_config: Option<&OsStr>) -> Vec<PathBuf> {
    if let Some(paths) = env_jj_config {
        return std::env::split_paths(paths)
            .filter(|path| !path.as_os_str().is_empty())
            .collect();
    }

    let home_dir = etcetera::home_dir()
        .ok()
        .map(|dir| dunce::canonicalize(&dir).unwrap_or(dir));
    let user_config_dir = etcetera::choose_base_strategy()
        .ok()
        .map(|strategy| strategy.config_dir());

    let home_config_path = home_dir.map(|dir| dir.join(".jjconfig.toml"));
    let platform_config_path = user_config_dir
        .clone()
        .map(|dir| dir.join("jj").join("config.toml"));
    let platform_config_dir =
        user_config_dir.map(|dir| dir.join("jj").join("conf.d"));

    let mut paths = Vec::new();
    match home_config_path {
        Some(path) if path.exists() || platform_config_path.is_none() => {
            paths.push(path);
        }
        Some(_) | None => {}
    }
    if let Some(path) = platform_config_path {
        paths.push(path);
    }
    if let Some(path) = platform_config_dir.filter(|path| path.exists()) {
        paths.push(path);
    }
    paths
}

/// Drives one of the `OpStore` trait's async reads to completion. There is no
/// runtime in this process, and these are plain file reads.
fn block_on_jj<T>(
    read: impl Future<Output = Result<T, OpStoreError>>,
) -> Result<T, Error> {
    pollster::block_on(read).map_err(|source| Error::JjOpStore {
        source: Box::new(source),
    })
}

/// Whether the head could not be peeled because nothing is committed yet, as
/// opposed to the repository being unreadable.
const fn is_unborn_head(error: &gix::reference::head_commit::Error) -> bool {
    matches!(
        error,
        gix::reference::head_commit::Error::Head(
            gix::reference::find::existing::Error::NotFound { .. }
        )
    )
}

/// The superproject working directory implied by a submodule's git directory.
///
/// Scans the `modules` components innermost outward, taking the first whose
/// parent opens as a repository. Both directions matter: at depth 2
/// (`<super>/.git/modules/mid/modules/leaf`) the innermost anchors on `mid`,
/// while a submodule added under a path containing `modules`
/// (`<super>/.git/modules/modules/foo`) has an innermost candidate that is not a
/// repository, so a bare `rposition` reports absence.
///
/// `opens` is fallible so an unopenable candidate short-circuits instead of the
/// scan continuing outward and returning a plausible wrong path.
fn superproject_of(
    git_dir: &Path,
    opens: impl Fn(&Path) -> Result<Option<PathBuf>, Error>,
) -> Result<Option<PathBuf>, Error> {
    let components: Vec<_> = git_dir.components().collect();
    let anchors = components
        .iter()
        .enumerate()
        .filter(|(_, component)| {
            matches!(component, Component::Normal(name) if *name == "modules")
        })
        .map(|(index, _)| index);

    for anchor in anchors.collect::<Vec<_>>().into_iter().rev() {
        let candidate: PathBuf = components[..anchor].iter().collect();
        if let Some(workdir) = opens(&candidate)? {
            return Ok(Some(workdir));
        }
    }
    Ok(None)
}

/// Opens the repository containing `start`, walking upward. `Ok(None)` only
/// when no repository was found at all.
fn discover_git(start: &Path) -> Result<Option<gix::Repository>, Error> {
    match gix::discover(start) {
        Ok(repository) => Ok(Some(repository)),
        Err(gix::discover::Error::Discover(
            gix::discover::upwards::Error::NoGitRepository { .. }
            | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling {
                ..
            }
            | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. },
        )) => Ok(None),
        Err(error) => Err(Error::Git {
            path: start.to_path_buf(),
            source: Box::new(error),
        }),
    }
}

/// Opens a repository at exactly `path`, performing no walk. `Ok(None)` only
/// when the path is not a repository.
fn open_git(path: &Path) -> Result<Option<gix::Repository>, Error> {
    match gix::open(path) {
        Ok(repository) => Ok(Some(repository)),
        Err(gix::open::Error::NotARepository { .. }) => Ok(None),
        Err(error) => Err(Error::Git {
            path: path.to_path_buf(),
            source: Box::new(error),
        }),
    }
}

/// Loads the jj workspace rooted at `root`. Only the not-found variants are
/// `Ok(None)`: a `.jj` that cannot be read is not the same as no jj here.
fn load_jj_workspace(
    root: &Path,
) -> Result<Option<Box<dyn jj_lib::workspace::WorkspaceLoader>>, Error> {
    match DefaultWorkspaceLoaderFactory.create(root) {
        Ok(loader) => Ok(Some(loader)),
        Err(
            WorkspaceLoadError::NoWorkspaceHere(_)
            | WorkspaceLoadError::RepoDoesNotExist(_),
        ) => Ok(None),
        Err(error) => Err(Error::Jj {
            path: root.to_path_buf(),
            source: Box::new(error),
        }),
    }
}

/// Makes `start` absolute before any walk.
///
/// The walks disagree otherwise: `gix::discover` absolutises against the process
/// cwd internally, while `walk_up` is lexical and stops after one directory for a
/// relative path, since `Path::new("sub").parent()` is `Some("")`.
fn absolutise(start: &Path) -> Result<PathBuf, Error> {
    absolute(start).map_err(|source| Error::Absolutise {
        path: start.to_path_buf(),
        source,
    })
}

fn canonicalise(path: &Path) -> Result<PathBuf, Error> {
    path.canonicalize().map_err(|source| Error::Canonicalise {
        path: path.to_path_buf(),
        source,
    })
}

/// Falls back to the uncanonicalised path, because the callers returning
/// `Option`/`PathBuf` cannot carry the distinction.
fn canonical(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(error) => {
            warn!(
                path = %path.display(),
                %error, "could not canonicalise the path"
            );
            path.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::superproject_of;
    use super::Error;

    /// A total probe over known git-dirs, so the derivation is exercised
    /// without building real submodule fixtures.
    fn opens(
        repositories: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&Path) -> Result<Option<PathBuf>, Error> {
        move |candidate| {
            Ok(repositories.iter().find_map(|(git_dir, workdir)| {
                (candidate == Path::new(git_dir))
                    .then(|| PathBuf::from(workdir))
            }))
        }
    }

    #[test]
    fn a_depth_one_submodule_anchors_on_the_superproject() {
        let found = superproject_of(
            Path::new("/tmp/super/.git/modules/mid"),
            opens(&[("/tmp/super/.git", "/tmp/super")]),
        );
        assert_eq!(found.ok().flatten(), Some(PathBuf::from("/tmp/super")));
    }

    #[test]
    fn a_depth_two_submodule_anchors_on_the_nearest_modules() {
        // The outermost `modules` would name /tmp/super, the superproject of
        // `mid` rather than of `leaf`.
        let found = superproject_of(
            Path::new("/tmp/super/.git/modules/mid/modules/leaf"),
            opens(&[
                ("/tmp/super/.git", "/tmp/super"),
                ("/tmp/super/.git/modules/mid", "/tmp/super/mid"),
            ]),
        );
        assert_eq!(found.ok().flatten(), Some(PathBuf::from("/tmp/super/mid")));
    }

    #[test]
    fn the_scan_continues_outward_past_a_candidate_that_is_not_a_repository() {
        // `/tmp/supm/.git/modules` is a plain directory, where a bare
        // rposition would stop and report absence.
        let found = superproject_of(
            Path::new("/tmp/supm/.git/modules/modules/foo"),
            opens(&[("/tmp/supm/.git", "/tmp/supm")]),
        );
        assert_eq!(found.ok().flatten(), Some(PathBuf::from("/tmp/supm")));
    }

    #[test]
    fn a_git_dir_with_no_modules_component_has_no_superproject() {
        let found = superproject_of(Path::new("/tmp/plain/.git"), opens(&[]));
        assert_eq!(found.ok().flatten(), None);
    }

    #[test]
    fn an_unopenable_candidate_short_circuits_rather_than_scanning_past() {
        // Why the probe is fallible: with a bool, a corrupt superproject is
        // indistinguishable from "not a repository" and the scan continues.
        let found = superproject_of(
            Path::new("/tmp/super/.git/modules/mid/modules/leaf"),
            |candidate| {
                if candidate == Path::new("/tmp/super/.git/modules/mid") {
                    return Err(Error::JjStoreLayout {
                        store: candidate.to_path_buf(),
                    });
                }
                Ok(Some(PathBuf::from("/tmp/super")))
            },
        );
        assert!(
            found.is_err(),
            "an unopenable candidate must not be scanned past"
        );
    }

    type TestError = Box<dyn std::error::Error>;

    fn origin_repo() -> Result<(tempfile::TempDir, PathBuf), TestError> {
        use vcs_test_support::hermetic::Hermetic;

        let dir = tempfile::Builder::new()
            .prefix("vcs-library-origin-")
            .tempdir()?;
        let env = Hermetic::rooted_at(dir.path())?;
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&root)?;
        env.git(&["init", "--quiet"], &root)?;
        Ok((dir, root))
    }

    #[test]
    fn a_configured_origin_is_reported() -> Result<(), TestError> {
        use vcs_test_support::hermetic::Hermetic;

        use super::InProcessProbe;

        let (dir, root) = origin_repo()?;
        let env = Hermetic::rooted_at(dir.path())?;
        env.git(
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/atomicinnovation/accelerator.git",
            ],
            &root,
        )?;

        let probe = InProcessProbe;
        assert_eq!(
            probe.origin_url(&root)?,
            Some(
                "https://github.com/atomicinnovation/accelerator.git"
                    .to_owned()
            )
        );
        Ok(())
    }

    #[test]
    fn no_origin_remote_is_reported_as_none() -> Result<(), TestError> {
        use super::InProcessProbe;

        let (_dir, root) = origin_repo()?;
        let probe = InProcessProbe;
        assert_eq!(probe.origin_url(&root)?, None);
        Ok(())
    }

    #[test]
    fn a_directory_with_no_repository_is_an_error() -> Result<(), TestError> {
        use super::InProcessProbe;

        let dir = tempfile::Builder::new()
            .prefix("vcs-library-origin-none-")
            .tempdir()?;

        let probe = InProcessProbe;
        assert!(
            probe.origin_url(dir.path()).is_err(),
            "a caller-supplied root that gix cannot open at all must be an \
             error, not a clean absence"
        );
        Ok(())
    }

    #[test]
    fn an_unreadable_repository_is_an_error() -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;

        use super::InProcessProbe;

        let (dir, root) = origin_repo()?;
        let git_dir = root.join(".git");
        let original_mode = std::fs::metadata(&git_dir)?.permissions().mode();
        std::fs::set_permissions(
            &git_dir,
            std::fs::Permissions::from_mode(0o000),
        )?;

        let probe = InProcessProbe;
        let result = probe.origin_url(&root);

        std::fs::set_permissions(
            &git_dir,
            std::fs::Permissions::from_mode(original_mode),
        )?;
        drop(dir);

        assert!(
            result.is_err(),
            "an unreadable .git directory must be an error, not a clean absence"
        );
        Ok(())
    }
}
