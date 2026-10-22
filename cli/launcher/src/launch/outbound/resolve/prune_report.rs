//! What a prune reports, shared by single-file and tree prune so neither
//! imports the other.

use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneStep {
    AboutToList,
    Listed,
    RootLockHeld,
    TreeLockHeld,
}

/// Sees a prune reach each step; a test pauses it there.
pub trait PruneSteps {
    fn reached(&self, step: PruneStep);
}

pub struct NoPruneSteps;

impl PruneSteps for NoPruneSteps {
    fn reached(&self, _step: PruneStep) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockFailure {
    pub lock: PathBuf,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableClaims {
    pub directory: PathBuf,
    pub kind: io::ErrorKind,
}

/// A removal that failed for any reason but the entry already being gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalFailure {
    pub path: PathBuf,
    pub kind: io::ErrorKind,
}
