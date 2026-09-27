//! The credential ports every composition root that resolves a credential
//! builds, in one place.

use std::path::Path;

use config_adapters::credentials::CredentialPorts;

use crate::runner::command_runner;
use crate::tracking::VcsConfigFileTracking;

/// The system environment and filesystem, the fail-closed tracking adapter,
/// and a runner judging the repository from `config_root` and `cwd`.
#[must_use]
pub fn credential_ports(config_root: &Path, cwd: &Path) -> CredentialPorts {
    CredentialPorts::system(
        Box::new(VcsConfigFileTracking),
        command_runner(config_root, cwd),
    )
}
