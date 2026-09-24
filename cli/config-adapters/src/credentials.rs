//! The production adapters for the credential ladder's ports, and the
//! project-rooted assembly of its context.

use std::ffi::OsString;
use std::io::Read as _;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use config::credentials::CommandPolicy;
use config::credentials::CredentialContext;
use config::credentials::Environment;
use config::credentials::FileFacts;
use config::credentials::FileState;
use config::credentials::Provenance;
use config::credentials::TokenCommandFailure;
use config::credentials::TokenCommandRunner;
use config::credentials::INSECURE_MARKER_RELATIVE;
use config::ConfigAccess;

/// Repo-relative path of the personal config the ladder's personal rungs read.
pub const PERSONAL_CONFIG_RELATIVE: &str = ".accelerator/config.local.md";

/// The ports one credential resolution runs against.
pub struct CredentialPorts {
    pub environment: Box<dyn Environment>,
    pub files: Box<dyn FileFacts>,
    pub commands: Box<dyn TokenCommandRunner>,
    pub provenance: Box<dyn Provenance>,
}

impl CredentialPorts {
    /// The production environment, filesystem, and helper runner, with the
    /// caller's VCS provenance, which needs a VCS adapter this crate does not
    /// depend on.
    #[must_use]
    pub fn system(provenance: Box<dyn Provenance>) -> Self {
        Self {
            environment: Box::new(SystemEnvironment),
            files: Box::new(SystemFileFacts),
            commands: Box::new(BashTokenCommandRunner),
            provenance,
        }
    }
}

/// The ladder's context for the project at `root`: its personal config and
/// insecure-local marker, and a helper rooted there under `command_timeout`.
#[must_use]
pub fn project_credential_context<'a>(
    root: &Path,
    ports: &'a CredentialPorts,
    config: &'a dyn ConfigAccess,
    command_timeout: Duration,
) -> CredentialContext<'a> {
    CredentialContext {
        environment: ports.environment.as_ref(),
        config,
        provenance: ports.provenance.as_ref(),
        files: ports.files.as_ref(),
        commands: ports.commands.as_ref(),
        personal_config: root.join(PERSONAL_CONFIG_RELATIVE),
        insecure_marker: root.join(INSECURE_MARKER_RELATIVE),
        command: CommandPolicy {
            timeout: command_timeout,
            ..CommandPolicy::rooted_at(root.to_path_buf())
        },
    }
}

/// The production environment.
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// The production filesystem.
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

/// The production helper runner: `bash -c` under [`CommandPolicy`], with a
/// scrubbed environment, a null stdin, and a discarded stderr.
pub struct BashTokenCommandRunner;

impl TokenCommandRunner for BashTokenCommandRunner {
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, TokenCommandFailure> {
        let scrubbed: Vec<(String, OsString)> = ["PATH", "HOME", "TERM"]
            .iter()
            .filter_map(|name| {
                std::env::var_os(name).map(|value| ((*name).to_owned(), value))
            })
            .collect();

        let mut child = Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&policy.working_directory)
            .env_clear()
            .envs(scrubbed)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                TokenCommandFailure::CouldNotRun(error.to_string())
            })?;

        let stdout = child.stdout.take();
        let cap = policy.max_output_bytes;
        let reader = std::thread::spawn(move || {
            let mut captured = Vec::new();
            if let Some(stream) = stdout {
                let mut bounded = stream.take(cap as u64);
                let _ = bounded.read_to_end(&mut captured);
            }
            captured
        });

        let deadline = Instant::now() + policy.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        break Err(TokenCommandFailure::TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => {
                    break Err(TokenCommandFailure::Failed(format!(
                        "could not be awaited: {error}"
                    )))
                }
            }
        };

        let captured = reader.join().unwrap_or_default();
        let status = status?;

        if !status.success() {
            return Err(TokenCommandFailure::Failed(format!(
                "exited with {status}"
            )));
        }

        let printed = String::from_utf8(captured).map_err(|_| {
            TokenCommandFailure::Failed("produced non-UTF-8 output".to_owned())
        })?;
        Ok(printed
            .strip_suffix('\n')
            .map_or_else(|| printed.clone(), str::to_owned))
    }
}
