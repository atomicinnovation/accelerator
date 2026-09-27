//! Whether a file carries changes version control could not restore.

/// Three-valued, so a failed probe and a clean tree are never one value.
///
/// `Unknown` decides as `Dirty` everywhere, since VCS revert cannot recover
/// the uncommitted working-copy changes an overwrite would destroy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirtiness {
    Clean,
    Dirty,
    Unknown,
}
