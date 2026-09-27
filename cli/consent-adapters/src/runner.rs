//! The one production command runner every composition root hands the
//! consent policy.

use std::path::Path;

use config::consent::Runner;
use config_adapters::credentials::BashCommandRunner;
use config_adapters::credentials::SystemEnvironment;

use crate::roots::repository_roots;

/// A runner judging the repository from `config_root` and `cwd`, reading the
/// process environment, and making working directories under the system
/// temporary directory.
#[must_use]
pub fn command_runner(config_root: &Path, cwd: &Path) -> Runner {
    Runner::new(Box::new(BashCommandRunner::new(
        repository_roots(config_root, cwd),
        Box::new(SystemEnvironment),
        std::env::temp_dir(),
    )))
}
