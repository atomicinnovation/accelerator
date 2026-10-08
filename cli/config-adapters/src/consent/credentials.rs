//! The credential ports every composition root that resolves a credential
//! builds, in one place.

use std::path::Path;

use config::consent::Runner;
use vcs::tracking::RepositoryTracking;

use super::roots::repository_roots;
use super::tracking::TrackedConfigFile;
use crate::credentials::BashCommandRunner;
use crate::credentials::CredentialPorts;
use crate::credentials::SystemEnvironment;

/// The system environment and filesystem, the fail-closed tracking adapter,
/// and a runner judging the repository from `config_root` and `cwd`.
#[must_use]
pub fn credential_ports(
    tracking: impl RepositoryTracking + 'static,
    config_root: &Path,
    cwd: &Path,
) -> CredentialPorts {
    let runner = command_runner(&tracking, config_root, cwd);
    CredentialPorts::system(Box::new(TrackedConfigFile(tracking)), runner)
}

/// Reads the process environment and makes working directories under the
/// system temporary directory.
fn command_runner(
    tracking: &dyn RepositoryTracking,
    config_root: &Path,
    cwd: &Path,
) -> Runner {
    Runner::new(Box::new(BashCommandRunner::new(
        repository_roots(tracking, config_root, cwd),
        Box::new(SystemEnvironment),
        std::env::temp_dir(),
    )))
}
