//! The production adapters for the credential ladder's ports, and the
//! project-rooted assembly of its context.

use std::path::Path;
use std::time::Duration;

use config::consent::CommandExecution;
use config::consent::ConfigFileTracking;
use config::consent::ProvenanceContext;
use config::consent::Runner;
use config::credentials::CredentialContext;
use config::credentials::Environment;
use config::ConfigAccess;

pub use crate::command_runner::BashCommandRunner;

/// Repo-relative path of the personal config the ladder's personal rungs read.
pub const PERSONAL_CONFIG_RELATIVE: &str = ".accelerator/config.local.md";

/// The ports one credential resolution runs against.
pub struct CredentialPorts {
    pub environment: Box<dyn Environment>,
    pub runner: Runner,
    pub tracking: Box<dyn ConfigFileTracking>,
}

impl CredentialPorts {
    /// The production environment, with the caller's tracking and command
    /// runner, which need a VCS adapter this crate does not depend on.
    #[must_use]
    pub fn system(
        tracking: Box<dyn ConfigFileTracking>,
        runner: Runner,
    ) -> Self {
        Self {
            environment: Box::new(SystemEnvironment),
            runner,
            tracking,
        }
    }
}

/// The ladder's context for the project at `root`: its personal config, and a
/// helper run under `command_timeout`.
#[must_use]
pub fn project_credential_context<'a>(
    root: &Path,
    ports: &'a CredentialPorts,
    config: &'a dyn ConfigAccess,
    command_timeout: Duration,
) -> CredentialContext<'a> {
    CredentialContext {
        provenance: ProvenanceContext {
            config,
            tracking: ports.tracking.as_ref(),
            environment: ports.environment.as_ref(),
            personal_config: root.join(PERSONAL_CONFIG_RELATIVE),
        },
        execution: CommandExecution {
            runner: &ports.runner,
            timeout: command_timeout,
        },
    }
}

pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}
