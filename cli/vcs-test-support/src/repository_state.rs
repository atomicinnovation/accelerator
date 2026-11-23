//! What a repository holds, projected so that two builds of the same script
//! compare equal whatever IDs and times each build gave its commits.

mod masking;
mod order;
mod reading;

use std::collections::BTreeMap;
use std::fs;
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
    pub references: BTreeMap<String, CommitPosition>,
    pub working_copy_parents: Vec<CommitPosition>,
    pub working_copy: BTreeMap<String, String>,
}

impl RepositoryState {
    /// Projects the repository at `root`, reading through `hermetic`.
    ///
    /// Reading never snapshots a jj working copy, and spawns at most two
    /// processes whatever the number of commits.
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
        let working_copy = working_copy(root, &masking)?;
        let position_of = |id: &String| {
            positions.get(id).copied().ok_or_else(|| {
                Error::message(format!(
                    "{id} names no commit the projection holds"
                ))
            })
        };

        let mut commits = raw
            .commits
            .iter()
            .map(|commit| {
                Ok(CommitState {
                    position: position_of(&commit.id)?,
                    parents: commit
                        .parents
                        .iter()
                        .map(position_of)
                        .collect::<Result<_, Error>>()?,
                    description: commit.description.clone(),
                    files: commit
                        .files
                        .iter()
                        .map(|(path, content)| {
                            (path.clone(), masking.mask(content))
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        commits.sort_by_key(|commit| commit.position);
        Ok(Self {
            kind: raw.kind,
            commits,
            references: raw
                .references
                .iter()
                .map(|(name, id)| Ok((name.clone(), position_of(id)?)))
                .collect::<Result<_, Error>>()?,
            working_copy_parents: raw
                .working_copy_parents
                .iter()
                .map(position_of)
                .collect::<Result<_, Error>>()?,
            working_copy,
        })
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
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitPosition(pub usize);

/// A commit as a reader finds it, before any ID is replaced by a position.
#[derive(Debug)]
struct RawCommit {
    id: String,
    change: Option<String>,
    parents: Vec<String>,
    description: String,
    files: BTreeMap<String, String>,
}

/// A repository as a reader finds it, naming commits by ID.
#[derive(Debug)]
struct RawRepository {
    kind: RepositoryKind,
    commits: Vec<RawCommit>,
    references: BTreeMap<String, String>,
    working_copy_parents: Vec<String>,
}

/// Every file on disk outside the repository's own metadata and the hermetic
/// environment's entries, keyed by its path beneath `root`.
fn working_copy(
    root: &Path,
    masking: &IdMasking,
) -> Result<BTreeMap<String, String>, Error> {
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
        let content = if file_type.is_symlink() {
            format!("symlink to {}", fs::read_link(entry.path())?.display())
        } else if file_type.is_file() {
            String::from_utf8_lossy(&fs::read(entry.path())?).into_owned()
        } else {
            continue;
        };
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| Error::message(error.to_string()))?;
        files.insert(relative.display().to_string(), masking.mask(&content));
    }
    Ok(files)
}

fn is_metadata(name: &std::ffi::OsStr) -> bool {
    name == ".git"
        || name == ".jj"
        || name.to_string_lossy().starts_with(ENTRY_PREFIX)
}
