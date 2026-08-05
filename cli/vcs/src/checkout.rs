//! The checkout facts `vcs-adapters::library::InProcessProbe`'s taxonomy
//! queries report, and the `classify` cascade (added in a later change)
//! composes over.

use std::path::PathBuf;

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
    pub git: Result<Option<PathBuf>, kernel::Error>,
    pub jj: Result<Option<PathBuf>, kernel::Error>,
}
