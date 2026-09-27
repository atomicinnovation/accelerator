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
use config::credentials::FileFacts;
use config::credentials::FileState;
use config::credentials::INSECURE_MARKER_RELATIVE;
use config::ConfigAccess;

pub use crate::command_runner::BashCommandRunner;

/// Repo-relative path of the personal config the ladder's personal rungs read.
pub const PERSONAL_CONFIG_RELATIVE: &str = ".accelerator/config.local.md";

/// The ports one credential resolution runs against.
pub struct CredentialPorts {
    pub environment: Box<dyn Environment>,
    pub files: Box<dyn FileFacts>,
    pub runner: Runner,
    pub tracking: Box<dyn ConfigFileTracking>,
}

impl CredentialPorts {
    /// The production environment and filesystem, with the caller's tracking
    /// and command runner, which need a VCS adapter this crate does not
    /// depend on.
    #[must_use]
    pub fn system(
        tracking: Box<dyn ConfigFileTracking>,
        runner: Runner,
    ) -> Self {
        Self {
            environment: Box::new(SystemEnvironment),
            files: Box::new(SystemFileFacts),
            runner,
            tracking,
        }
    }
}

/// The ladder's context for the project at `root`: its personal config and
/// insecure-local marker, and a helper run under `command_timeout`.
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
        files: ports.files.as_ref(),
        insecure_marker: root.join(INSECURE_MARKER_RELATIVE),
    }
}

pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

pub struct SystemFileFacts;

impl FileFacts for SystemFileFacts {
    fn inspect(&self, path: &Path) -> Result<FileState, String> {
        let facts = match std::fs::symlink_metadata(path) {
            Ok(facts) => facts,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(FileState::Absent)
            }
            Err(error) => return Err(error.to_string()),
        };
        let file_type = facts.file_type();
        if file_type.is_symlink() {
            return Ok(if path.exists() {
                FileState::Symlink
            } else {
                FileState::Absent
            });
        }
        if file_type.is_file() {
            return Ok(FileState::File {
                mode: file_mode(&facts),
            });
        }
        Ok(FileState::Other)
    }
}

#[cfg(unix)]
fn file_mode(facts: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt as _;
    facts.mode() & 0o7777
}

#[cfg(not(unix))]
const fn file_mode(_facts: &std::fs::Metadata) -> u32 {
    0o600
}
