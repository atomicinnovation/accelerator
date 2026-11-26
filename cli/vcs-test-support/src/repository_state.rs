//! What a repository holds, projected so that two builds of the same script
//! compare equal whatever IDs and times each build gave its commits.

mod masking;
mod order;
mod reading;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use walkdir::WalkDir;

use crate::colocation::JjColocation;
use crate::hermetic::Hermetic;
use crate::hermetic::ENTRY_PREFIX;
use crate::Error;
use masking::IdMasking;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryState {
    pub kind: RepositoryKind,
    pub commits: Vec<CommitState>,
    pub bookmarks: BTreeMap<String, CommitPosition>,
    pub git: Option<GitSide>,
    pub working_copy_parents: Vec<CommitPosition>,
    pub working_copy: BTreeMap<String, FileState>,
}

/// What git records beside the commits, for a git repository or a jj one
/// colocated with git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSide {
    pub head: Head,
    pub references: BTreeMap<String, CommitPosition>,
    pub index: BTreeMap<String, FileState>,
}

/// Whether git's next commit advances a branch, which `HEAD`'s commit in
/// [`GitSide::references`] cannot say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Attached(String),
    Detached,
}

impl RepositoryState {
    /// Projects the repository at `root`, reading through `hermetic`.
    ///
    /// Reading never snapshots a jj working copy. It spawns a fixed number
    /// of processes whatever the number of commits: two for a pure jj
    /// repository, three for git, and four for colocated jj.
    ///
    /// # Errors
    ///
    /// When `root` is no repository, git or jj cannot read it, or two of its
    /// commits cannot be told apart.
    pub fn of(root: &Path, hermetic: &Hermetic) -> Result<Self, Error> {
        let raw = match RepositoryKind::of(root)? {
            RepositoryKind::Git => reading::git(root, hermetic)?,
            RepositoryKind::Jj(colocation) => {
                reading::jj(root, hermetic, colocation)?
            }
        };
        let positions = order::ordered_positions(&raw)?;
        let masking = IdMasking::of(&raw.commits, &positions);
        let projection = Projection {
            positions: &positions,
            masking: &masking,
        };

        let mut commits = raw
            .commits
            .iter()
            .map(|commit| {
                Ok(CommitState {
                    position: projection.position_of(&commit.id)?,
                    parents: projection.positions_of(&commit.parents)?,
                    description: commit.description.clone(),
                    files: projection.files(&commit.files),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        commits.sort_by_key(|commit| commit.position);
        Ok(Self {
            kind: raw.kind,
            commits,
            bookmarks: projection.references(&raw.bookmarks)?,
            git: raw
                .git
                .as_ref()
                .map(|git| {
                    Ok::<_, Error>(GitSide {
                        head: git.head.clone(),
                        references: projection.references(&git.references)?,
                        index: projection.files(&git.index),
                    })
                })
                .transpose()?,
            working_copy_parents: projection
                .positions_of(&raw.working_copy_parents)?,
            working_copy: projection.files(&working_copy(root)?),
        })
    }
}

/// Replaces the IDs a reader found with positions two builds agree on.
struct Projection<'p> {
    positions: &'p BTreeMap<String, CommitPosition>,
    masking: &'p IdMasking,
}

impl Projection<'_> {
    fn position_of(&self, id: &String) -> Result<CommitPosition, Error> {
        self.positions.get(id).copied().ok_or_else(|| {
            Error::message(format!("{id} names no commit the projection holds"))
        })
    }

    fn positions_of(
        &self,
        ids: &[String],
    ) -> Result<Vec<CommitPosition>, Error> {
        ids.iter().map(|id| self.position_of(id)).collect()
    }

    fn references(
        &self,
        references: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, CommitPosition>, Error> {
        references
            .iter()
            .map(|(name, id)| Ok((name.clone(), self.position_of(id)?)))
            .collect()
    }

    fn files(
        &self,
        files: &BTreeMap<String, FileState>,
    ) -> BTreeMap<String, FileState> {
        files
            .iter()
            .map(|(path, file)| (path.clone(), file.masked(self.masking)))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryKind {
    Git,
    Jj(JjColocation),
}

impl RepositoryKind {
    fn of(root: &Path) -> Result<Self, Error> {
        let has_git = root.join(".git").exists();
        if root.join(".jj").is_dir() {
            Ok(Self::Jj(if has_git {
                JjColocation::Colocated
            } else {
                JjColocation::Pure
            }))
        } else if has_git {
            Ok(Self::Git)
        } else {
            Err(Error::message(format!(
                "{} holds neither a git nor a jj repository",
                root.display()
            )))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitState {
    pub position: CommitPosition,
    pub parents: Vec<CommitPosition>,
    pub description: String,
    pub files: BTreeMap<String, FileState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitPosition(pub usize);

/// A file as git records it: its kind and, for a symlink, its target as
/// content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileState {
    pub kind: FileKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileKind {
    Regular,
    Executable,
    Symlink,
}

impl FileState {
    fn masked(&self, masking: &IdMasking) -> Self {
        Self {
            kind: self.kind,
            content: masking.mask(&self.content),
        }
    }
}

/// A commit as a reader finds it, before any ID is replaced by a position.
#[derive(Debug)]
struct RawCommit {
    id: String,
    change: Option<String>,
    parents: Vec<String>,
    description: String,
    files: BTreeMap<String, FileState>,
}

/// A repository as a reader finds it, naming commits by ID.
#[derive(Debug)]
struct RawRepository {
    kind: RepositoryKind,
    commits: Vec<RawCommit>,
    bookmarks: BTreeMap<String, String>,
    git: Option<RawGitSide>,
    working_copy_parents: Vec<String>,
}

#[derive(Debug)]
struct RawGitSide {
    head: Head,
    references: BTreeMap<String, String>,
    index: BTreeMap<String, FileState>,
}

/// The bit git reads to record a file as executable.
const OWNER_EXECUTE: u32 = 0o100;

/// Every file on disk outside the repository's own metadata and the hermetic
/// environment's entries, keyed by its path beneath `root`.
fn working_copy(root: &Path) -> Result<BTreeMap<String, FileState>, Error> {
    let mut files = BTreeMap::new();
    let walk =
        WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() > 1 || !is_metadata(entry.file_name())
            });
    for entry in walk {
        let entry = entry.map_err(|error| Error::message(error.to_string()))?;
        let file_type = entry.file_type();
        let file = if file_type.is_symlink() {
            FileState {
                kind: FileKind::Symlink,
                content: fs::read_link(entry.path())?.display().to_string(),
            }
        } else if file_type.is_file() {
            let executable = entry.path().metadata()?.permissions().mode()
                & OWNER_EXECUTE
                != 0;
            FileState {
                kind: if executable {
                    FileKind::Executable
                } else {
                    FileKind::Regular
                },
                content: String::from_utf8_lossy(&fs::read(entry.path())?)
                    .into_owned(),
            }
        } else {
            continue;
        };
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| Error::message(error.to_string()))?;
        files.insert(relative.display().to_string(), file);
    }
    Ok(files)
}

fn is_metadata(name: &std::ffi::OsStr) -> bool {
    name == ".git"
        || name == ".jj"
        || name.to_string_lossy().starts_with(ENTRY_PREFIX)
}
