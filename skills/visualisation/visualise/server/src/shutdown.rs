//! Shared shutdown plumbing: the reason enum and the mpsc
//! channel type used to converge multiple triggers into one
//! deterministic shutdown path. Neither `server` nor
//! `lifecycle` depends on the other; both depend on this.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShutdownReason {
    Sigterm,
    Sigint,
    OwnerPidExited,
    IdleTimeout,
    StartupFailure,
    ForcedSigkill,
}
