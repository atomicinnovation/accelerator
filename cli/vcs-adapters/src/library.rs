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

use std::fmt;
use std::fs;
use std::future::Future;
use std::path::absolute;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use jj_lib::backend::CommitId;
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
use vcs::RepoRoot;
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
            | Self::JjOpStore { source } => Some(source.as_ref()),
            Self::JjStoreLayout { .. } | Self::JjWorkingCopyBackend { .. } => {
                None
            }
        }
    }
}

/// Whether a checkout is a linked worktree, and where its git directories are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeFacts {
    pub linked: bool,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    /// `None` for a bare repository.
    pub main_worktree_root: Option<PathBuf>,
}

/// Whether a jj workspace owns its repository store or shares another's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JjWorkspaceRole {
    Main,
    Secondary,
}

/// Which jj repository a workspace belongs to, and in what role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjRepositoryFacts {
    pub role: JjWorkspaceRole,
    pub main_root: PathBuf,
}

/// The git repository root and the jj workspace root, each resolved by its own
/// walk so neither is truncated by the other's marker.
///
/// A `Result` per side rather than one for the struct, so a git-side failure
/// cannot be observed as "jj only". Compare the sides only when both are `Ok`;
/// an `Err` means "not comparable", not "unequal".
#[derive(Debug)]
pub struct DualRoots {
    pub git: Result<Option<PathBuf>, Error>,
    pub jj: Result<Option<PathBuf>, Error>,
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
            git: git_root(start),
            jj: self.jj_workspace_root(start),
        }
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
}
