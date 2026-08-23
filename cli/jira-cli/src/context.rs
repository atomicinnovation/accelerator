//! The credential context, the client builder, and the validated base-URL seam.
//!
//! The `ACCELERATOR_JIRA_API_URL` override ships in the release binary but is
//! admitted only as a validated credential destination (Decision 10): the
//! release path routes it through Jira's own strict `auth::base_url` (https, no
//! userinfo/port/query/fragment, an `*.atlassian.net`/allowlisted host),
//! **unchanged**. A loopback mock is reached only under the test-only
//! `test-loopback` feature, by a dedicated branch that points the resolved
//! `Credentials` at the override directly and never touches `base_url`. A
//! set-but-inadmissible value is a hard usage error, never a silent
//! fall-through to `from_config`.

use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess;
use config::Key;
use config_adapters::compose;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use jira_client::auth::base_url;
use jira_client::auth::project_code;
use jira_client::auth::resolve_credentials;
use jira_client::jql::FixedResolver;
use jira_client::transport::Transport;
use jira_client::transport::Url;
use jira_client::ClientError;
use jira_client::JiraClient;
use tracker_support::ClockJitter;
use tracker_support::CommandPolicy;
use tracker_support::CredentialContext;
use tracker_support::Provenance;
use tracker_support::SystemEnvironment;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs_adapters::library::InProcessProbe;

/// Why a client could not be built, mapped to an exit code by the caller.
pub enum ContextError {
    /// `ACCELERATOR_JIRA_API_URL` is set but unparseable or non-admissible.
    BadApiUrl(String),
    /// Config could not be composed, or a required path could not be resolved.
    Config(String),
    /// The client could not be built from configuration.
    Client(ClientError),
}

/// The env override, resolved to a validated destination `Url`.
///
/// Release admits it only through the strict `base_url`; `test-loopback` admits
/// a loopback mock URL verbatim, gated behind the feature so no ordinary build
/// carries the relaxation.
fn api_base_uri() -> Result<Option<Url>, ContextError> {
    let Some(raw) = std::env::var_os("ACCELERATOR_JIRA_API_URL") else {
        return Ok(None);
    };
    let raw = raw.to_string_lossy().into_owned();
    if cfg!(feature = "test-loopback") {
        let url = Url::parse(&raw)
            .map_err(|_| ContextError::BadApiUrl(raw.clone()))?;
        return Ok(Some(url));
    }
    // Release: the override must clear the same strict destination bar a
    // configured `jira.site` does — https, no userinfo/port/query/fragment, an
    // `*.atlassian.net` host — with no loopback relaxation.
    base_url(&raw, &[])
        .map(Some)
        .map_err(|_| ContextError::BadApiUrl(raw))
}

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
        InProcessProbe
            .is_tracked(&self.root, relpath, self.kind)
            .unwrap_or(false)
    }
}

fn integrations_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, ContextError> {
    let key = Key::parse("paths.integrations")
        .map_err(|error| ContextError::Config(error.to_string()))?;
    let relative = config
        .effective_nonempty(&key, None)
        .map_err(|error| ContextError::Config(error.to_string()))?
        .rendered();
    let path = Path::new(&relative);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}

/// A built client and the paths its init caches are written under.
pub struct Built {
    pub client: JiraClient,
    /// `paths.integrations` — the Jira state dir is `<root>/jira/`.
    pub integrations_root: PathBuf,
    /// The discovered project root, the write-bounds ceiling for the caches.
    pub project_root: PathBuf,
}

/// Builds a Jira client from the working directory's configuration, honouring
/// the validated base-URL seam, and returns it alongside the paths the init
/// caches are written under.
///
/// # Errors
///
/// [`ContextError`] for a bad override, an unreadable config, or a client that
/// cannot be constructed.
pub fn build_client() -> Result<Built, ContextError> {
    let start = std::env::current_dir().map_err(|error| {
        ContextError::Config(format!(
            "could not read the working directory: {error}"
        ))
    })?;
    let composed = compose(&start, LegacyPolicy::Reject)
        .map_err(|error| ContextError::Config(error.to_string()))?;
    let service: &dyn ConfigAccess = &composed.service;
    let root = FileConfigStore::discover_root(&start);
    let integrations_root = integrations_dir(service, &root)?;

    let environment = SystemEnvironment;
    let provenance = VcsProvenance::discovered(root.clone());
    let context = CredentialContext {
        environment: &environment,
        config: service,
        provenance: &provenance,
        personal_config: root.join(".accelerator/config.local.md"),
        insecure_marker: root.join(".claude/insecure-local-ok"),
        command: CommandPolicy::rooted_at(root.clone()),
    };

    let client = match api_base_uri()? {
        Some(endpoint) => build_with_override(&context, endpoint)
            .map_err(ContextError::Client)?,
        None => {
            JiraClient::from_config(&context).map_err(ContextError::Client)?
        }
    };
    Ok(Built {
        client,
        integrations_root,
        project_root: root,
    })
}

/// The override branch: the whole client `from_config` builds, differing only
/// in the base URL the resolved credentials point at.
fn build_with_override(
    context: &CredentialContext<'_>,
    endpoint: Url,
) -> Result<JiraClient, ClientError> {
    let mut credentials = resolve_credentials(context)?;
    credentials.base = endpoint;
    let project = project_code(context.config)?;
    let transport = Transport::new(
        credentials,
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )?;
    Ok(JiraClient::new(
        transport,
        project,
        Box::new(FixedResolver::new()),
        Box::new(FixedResolver::new()),
    ))
}
