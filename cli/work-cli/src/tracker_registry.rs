//! Resolves a `RemoteTracker` from the `work.integration` config key.

use std::path::Path;
use std::path::PathBuf;

use config::credentials::CommandPolicy;
use config::credentials::Provenance;
use config::ConfigAccess;
use config_adapters::credentials::project_credential_context;
use config_adapters::credentials::CredentialPorts;
use jira_client::JiraClient;
use linear_client::LinearClient;
use tracker::RemoteTracker;
use tracker_support::TransportConfig;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs_adapters::library::InProcessProbe;

pub enum SelectionError {
    Unset,
    Unrecognised {
        name: String,
    },
    NotAvailable {
        name: String,
    },
    Unconfigured {
        name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl SelectionError {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Unset => "work.integration is not set. Recognised \
                trackers: linear, jira, trello, github-issues. Configure \
                one with /accelerator:configure."
                .to_owned(),
            Self::Unrecognised { name } => format!(
                "work.integration names an unrecognised tracker: '{name}'. \
                 Recognised trackers: linear, jira, trello, github-issues."
            ),
            Self::NotAvailable { name } => format!(
                "work.integration names '{name}', which is recognised but \
                 has no client wired yet."
            ),
            // Rendered from the source chain, not a stored string, so the
            // caller can still tell a missing site from a failed credential
            // helper from a shared-config refusal.
            Self::Unconfigured { name, source } => format!(
                "work.integration names '{name}', which is wired but not \
                 usable — its configuration or credentials are missing or \
                 refused: {source}"
            ),
        }
    }
}

pub trait TrackerRegistry {
    /// # Errors
    ///
    /// [`SelectionError`] when `name` is empty, unrecognised, names a tracker
    /// with no client built, or names a wired tracker whose configuration or
    /// credentials cannot be resolved.
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError>;
}

/// Answers `Provenance` for the credential trust boundary by reading whether a
/// path is tracked in the repository at `root`, in-process.
///
/// A read failure is treated as untracked, so an unreadable repository
/// loosens the check rather than refusing every credential.
struct VcsProvenance {
    root: PathBuf,
    kind: VcsKind,
}

impl VcsProvenance {
    fn discovered(root: PathBuf) -> Self {
        let kind = InProcessProbe.kind(&root);
        Self { root, kind }
    }
}

impl Provenance for VcsProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        let Ok(relpath) = path.strip_prefix(&self.root) else {
            return false;
        };
        let Some(relpath) = relpath.to_str() else {
            return false;
        };
        match InProcessProbe.is_tracked(&self.root, relpath, self.kind) {
            Ok(tracked) => tracked,
            Err(error) => {
                tracing::warn!(
                    %error,
                    path = %path.display(),
                    "could not determine VCS tracking; treating as untracked"
                );
                false
            }
        }
    }
}

/// The production registry. `jira` and `linear` resolve real clients from
/// configuration; `trello` and `github-issues` report not-available; everything
/// else falls to `Unrecognised`.
pub struct ConfiguredTrackers<'a> {
    config: &'a dyn ConfigAccess,
    root: PathBuf,
}

impl<'a> ConfiguredTrackers<'a> {
    #[must_use]
    pub const fn new(config: &'a dyn ConfigAccess, root: PathBuf) -> Self {
        Self { config, root }
    }

    fn unconfigured(
        name: &str,
        error: impl std::error::Error + Send + Sync + 'static,
    ) -> SelectionError {
        SelectionError::Unconfigured {
            name: name.to_owned(),
            source: Box::new(error),
        }
    }

    /// The transport bounds for `name`, with the page caps sourced from the
    /// `<name>.pull.max_pages` block. The block is already validated on the
    /// sync path before a client is resolved, so a fault here is defensive.
    fn transport_config(
        &self,
        name: &str,
    ) -> Result<TransportConfig, SelectionError> {
        let ceilings =
            tracker_support::pull::resolve_ceilings(self.config, name)
                .map_err(|detail| {
                    Self::unconfigured(name, PullError(detail))
                })?;
        Ok(TransportConfig {
            discovery_max_pages: ceilings.discovery_pages,
            keyed_read_max_pages: ceilings.keyed_read_pages,
            ..TransportConfig::default()
        })
    }
}

/// A `<tracker>.pull` resolution fault, carrying its already-rendered message
/// so it can route through [`SelectionError::Unconfigured`]'s error source.
#[derive(Debug)]
struct PullError(String);

impl std::fmt::Display for PullError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PullError {}

impl TrackerRegistry for ConfiguredTrackers<'_> {
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
        let ports = CredentialPorts::system(Box::new(
            VcsProvenance::discovered(self.root.clone()),
        ));
        let context = project_credential_context(
            &self.root,
            &ports,
            self.config,
            CommandPolicy::DEFAULT_TIMEOUT,
        );
        match name {
            "" => Err(SelectionError::Unset),
            "jira" => {
                let transport_config = self.transport_config(name)?;
                JiraClient::from_config(&context, transport_config)
                    .map(|client| Box::new(client) as Box<dyn RemoteTracker>)
                    .map_err(|error| Self::unconfigured(name, error))
            }
            "linear" => {
                let integrations_root =
                    crate::sync::integrations_dir(self.config, &self.root)
                        .map_err(|error| Self::unconfigured(name, error))?;
                let transport_config = self.transport_config(name)?;
                LinearClient::from_config(
                    &context,
                    &integrations_root,
                    transport_config,
                )
                .map(|client| Box::new(client) as Box<dyn RemoteTracker>)
                .map_err(|error| Self::unconfigured(name, error))
            }
            "trello" | "github-issues" => Err(SelectionError::NotAvailable {
                name: name.to_owned(),
            }),
            other => Err(SelectionError::Unrecognised {
                name: other.to_owned(),
            }),
        }
    }
}
