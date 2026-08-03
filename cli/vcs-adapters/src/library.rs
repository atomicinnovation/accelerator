//! The library-backed probe: git through `gix`, jj through `jj-lib`, both read
//! in the calling process rather than by spawning the VCS binaries.
//!
//! Two mechanisms live here and must not be confused. `RepoRoot::discover` is
//! the marker walk followed by nothing else, because the checkout boundary is
//! the start path or its nearest marked ancestor and never an ancestor above
//! it. `gix::discover` deliberately *does* walk past that boundary and is used
//! only where following a recorded link out of the checkout is the answer being
//! asked for. A ceiling cannot enforce the boundary rule — `ceiling_dirs`
//! computes its height as `strip_prefix(ceiling).components().count()` and
//! discards height 0, so a ceiling at the boundary is silently ignored.
//!
//! Every path returned from this module is canonicalised, at the single choke
//! point below. The sources disagree otherwise: `repo_path()` arrives already
//! canonicalised from jj-lib while `workspace_root()` is whatever was passed
//! in, and a linked worktree's `workdir()` is reconstructed from the absolute
//! path git recorded at `git worktree add` time.
//!
//! Three walks live here, and using the wrong one is the mistake this module
//! is shaped to prevent. The combined `.jj`-or-`.git` boundary walk answers
//! `RepoRoot::discover` only. A **`.jj`-only** walk answers the jj queries,
//! because `DefaultWorkspaceLoaderFactory::create` performs no walk of its own
//! and feeding it the combined boundary makes it report absence on a git
//! checkout nested inside a jj workspace — where `jj workspace root` reports a
//! root. `gix::discover` performs the third, for the git queries.
//!
//! Queries distinguish failure from absence: `Ok(None)` is "no repository of
//! this kind here", `Err` is "a repository is here and the pinned library could
//! not answer". Collapsing the two would be a real regression against the
//! subprocess probe, which runs its parse in a child process with a time cap
//! and a scrubbed environment; this module parses repository-controlled data in
//! the caller's address space with no time bound and no crash isolation. Only
//! the not-found-shaped variant of each library error maps to `Ok(None)`.
//!
//! A cargo-pup rule (`vcs_adapters_library_reads_in_process`) restricts this
//! module's imports to a permit list and denies `std::process`. cargo-pup
//! resolves a grouped `use a::{b, c}` to an empty module name, which the permit
//! list rejects — so every import here is single-item, and must stay that way.

use std::fmt;
use std::path::absolute;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use jj_lib::workspace::DefaultWorkspaceLoaderFactory;
use jj_lib::workspace::WorkspaceLoadError;
use jj_lib::workspace::WorkspaceLoaderFactory as _;
use tracing::warn;
use vcs::RepoRoot;
use vcs::VcsKind;
use vcs::VcsProbe;

use crate::markers::carries_any_marker;
use crate::markers::carries_jj_marker;
use crate::markers::marker_kind;
use crate::markers::walk_up;

/// A repository is here and the pinned library could not answer.
///
/// Deliberately not `kernel::Error`: `kernel` is not a dependency of this
/// crate, and taking one to carry an error type would couple the adapter to the
/// launcher's error vocabulary for no gain.
#[derive(Debug)]
pub enum Error {
    /// A path could not be made absolute, so the three walks would disagree.
    Absolutise {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A path could not be canonicalised, so it cannot be compared with the
    /// others this module returns.
    Canonicalise {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A git repository was found but could not be read.
    Git {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A jj workspace was found but could not be read.
    Jj {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A jj store resolved to something that is not a repository layout —
    /// either with no room for a root above it, or failing the
    /// `<root>/.jj/repo` post-condition.
    JjStoreLayout { store: PathBuf },
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
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Absolutise { source, .. }
            | Self::Canonicalise { source, .. } => Some(source),
            Self::Git { source, .. } | Self::Jj { source, .. } => {
                Some(source.as_ref())
            }
            Self::JjStoreLayout { .. } => None,
        }
    }
}

/// Whether a checkout is a linked worktree, and where its git directories are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeFacts {
    /// The `--git-dir` vs `--git-common-dir` comparison, canonicalised. `kind()`
    /// is not used for this: it is a single mutually-exclusive enum, so it
    /// cannot represent a checkout that is both a submodule and a linked
    /// worktree, which git reports as a worktree.
    pub linked: bool,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
    /// `None` for a bare repository. Carries no oracle for the bare and
    /// submodule shapes, where the shell's `dirname <common-dir>` formula does
    /// not name a worktree root at all.
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
/// Infallible as a whole, with a `Result` per side. A whole-struct `Result`
/// could not say "the git side failed but the jj side answered": a one-sided
/// failure would either propagate as `Err` and discard a valid answer, or
/// flatten to `None` and reinstate the absence/failure conflation on the single
/// field that separates a colocated checkout from a nested one. A repository
/// whose git side the pinned library cannot parse must never be observable as
/// "jj only".
///
/// Callers comparing the two sides for equality must treat any `Err` as "not
/// comparable", never as inequality.
#[derive(Debug)]
pub struct DualRoots {
    pub git: Result<Option<PathBuf>, Error>,
    pub jj: Result<Option<PathBuf>, Error>,
}

/// Reads a repository's root, idiom and revision in-process.
///
/// Ships unwired: `crate::facts` still composes the subprocess pair, and no
/// feature flag or config switch routes a caller here.
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
        // The raw value carries `../..` for a linked worktree, where the oracle
        // reports the resolved path; they are equal only after this.
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
    /// gix exposes no API for this direction — `main_repo()` on a submodule
    /// returns the submodule, and `submodules()` resolves superproject to
    /// children — so it is derived from the `git_dir()` path shape.
    ///
    /// # Errors
    ///
    /// When a candidate superproject directory exists but cannot be opened,
    /// which must not be mistaken for "this candidate is not a repository" and
    /// silently scanned past.
    pub fn superproject(&self, start: &Path) -> Result<Option<PathBuf>, Error> {
        let start = absolutise(start)?;
        let Some(repository) = discover_git(&start)? else {
            return Ok(None);
        };

        // A linked worktree *of* a submodule carries `modules` in its git dir,
        // and git reports **no** superproject for it. `kind()` cannot express
        // that: it is a single mutually-exclusive enum and reports `Submodule`
        // for exactly that shape, which is why the gate is the oracle's own
        // discriminator — the git-dir vs common-dir comparison — and not
        // `kind()`. Gating on `kind() == Submodule` instead both admits this
        // shape wrongly and rejects a submodule inside a linked worktree, whose
        // git dir sits under `worktrees/<id>/modules/` and which git *does*
        // give a superproject.
        let git_dir = canonicalise(repository.git_dir())?;
        if git_dir != canonicalise(repository.common_dir())? {
            return Ok(None);
        }

        // Canonicalisation belongs to the probe rather than to the scan, so the
        // scan stays pure path logic and its unit tests can drive it over paths
        // that need not exist.
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
    /// jj repository layout — the case a `.jj/repo` pointer aimed at an
    /// arbitrary existing directory produces, which would otherwise yield a
    /// real-looking but wrong repository root.
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

        // jj-lib canonicalises repo_path() itself but returns workspace_root()
        // as passed in, so both sides need it before comparison.
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
        // The shell oracle carries this post-condition explicitly, so a future
        // jj layout change cannot silently produce a wrong-but-non-empty root.
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
            VcsKind::Jj => {
                warn!(
                    vcs = "jj",
                    "jj-lib 0.43 exposes no read-only, settings-free route to \
                     the working-copy commit id, so no revision is reported"
                );
                None
            }
            VcsKind::None => None,
        }
    }
}

/// The repository a jj working copy belongs to, resolved through the loader so
/// the `.jj/repo`-file-means-secondary rule has one implementation.
///
/// The store is always `<repository>/.jj/repo`, so the repository is its
/// grandparent — for a secondary workspace that is the main repository, and for
/// a main workspace it is the workspace itself. `None` when this is not a jj
/// workspace at all, which is the one absence that does not log.
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

/// The full working-copy revision of a git checkout. `None` — unlogged — for a
/// repository with no commits yet, since that is a legitimate absence rather
/// than a probe that could not answer.
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

/// Whether the head could not be peeled because nothing has been committed yet,
/// as opposed to the repository being unreadable.
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
/// The rule is: scan the `modules` components from the **innermost outward**
/// and take the first whose parent opens as a repository. Two shapes
/// discriminate it. At submodule depth 2 the git dir is
/// `<super>/.git/modules/mid/modules/leaf`, and the innermost `modules` anchors
/// on the `mid` submodule — taking the outermost would name the wrong
/// repository. A submodule added at a path that itself contains `modules`
/// gives `<super>/.git/modules/modules/foo`, where the innermost candidate is
/// not a repository at all and the scan must continue outward; a bare
/// `rposition` stops there and reports absence.
///
/// `opens` is injected and **fallible** so the unit tests can drive the
/// derivation over known paths without building the matrix's most expensive
/// fixtures. It must not collapse to a bool: "this candidate is not a
/// repository" and "this candidate is a repository the pinned library could not
/// open" have to stay distinct, or a corrupt superproject makes the scan
/// continue outward and return a plausible wrong path.
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

/// Loads the jj workspace rooted at `root`. `Ok(None)` only for the two
/// not-found-shaped variants; every other failure is `Err`, because a `.jj`
/// directory that cannot be read is not the same as no jj repository here.
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
/// The three walks disagree otherwise: `gix::discover` absolutises against the
/// process cwd internally, whereas `walk_up` is purely lexical — for a relative
/// `"sub"`, `Path::new("sub").parent()` is `Some("")` and that parent is
/// `None`, so it tests one directory and stops. Given a relative start, a
/// colocated checkout would report a git root and no jj root: the wrong arm.
fn absolutise(start: &Path) -> Result<PathBuf, Error> {
    absolute(start).map_err(|source| Error::Absolutise {
        path: start.to_path_buf(),
        source,
    })
}

/// The fallible form of the canonicalisation choke point.
fn canonicalise(path: &Path) -> Result<PathBuf, Error> {
    path.canonicalize().map_err(|source| Error::Canonicalise {
        path: path.to_path_buf(),
        source,
    })
}

/// The single choke point every path leaving this module passes through.
///
/// Falls back to the uncanonicalised path rather than dropping the answer: the
/// callers return `Option`/`PathBuf` and cannot carry the distinction, and the
/// paths reaching here have just been read off the filesystem.
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

    /// A total probe over a known set of repository git-dirs, so the derivation
    /// is exercised without building the matrix's most expensive fixtures.
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
        // Taking the *outermost* `modules` would name /tmp/super, which is the
        // superproject of `mid` rather than of `leaf`.
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
        // A submodule added at a path that itself contains `modules` gives
        // `.git/modules/modules/foo`. The innermost candidate,
        // `/tmp/supm/.git/modules`, is a plain directory — a bare rposition
        // stops there and reports absence.
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
        // The reason the probe is fallible. With a bool return, "not a
        // repository" and "a repository the pinned library could not open" are
        // indistinguishable, so a corrupt superproject would let the scan
        // continue outward and return a plausible wrong path.
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
