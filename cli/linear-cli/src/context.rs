//! The credential context, the client builder, and the validated base-URL seam.
//!
//! The `ACCELERATOR_LINEAR_API_URL` override ships in the release binary but is
//! admitted only as a validated credential destination (Decision 10): Linear's
//! own `url_is_allowed` enforces https and a `*.linear.app` host, and a loopback
//! destination is admitted only under the test-only `test-loopback` feature,
//! never `debug_assertions`. A set-but-inadmissible value is a hard usage error,
//! never a silent fall-through to `from_config`.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use config::ConfigAccess;
use config::Key;
use config_adapters::compose;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use linear_client::auth::catalogue_team_key;
use linear_client::auth::resolve_credentials;
use linear_client::catalogue::CatalogueStates;
use linear_client::transport::Transport;
use linear_client::upload::url_is_allowed;
use linear_client::upload::UploadTransport;
use linear_client::ClientError;
use linear_client::LinearClient;
use reqwest::Url;
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
    /// `ACCELERATOR_LINEAR_API_URL` is set but unparseable or non-admissible.
    BadApiUrl(String),
    /// Config could not be composed, or a required path could not be resolved.
    Config(String),
    /// The client could not be built from configuration.
    Client(ClientError),
}

/// The env override, admitted only through Linear's complete https-destination
/// check with loopback gated behind `test-loopback`.
fn api_base_uri() -> Result<Option<Url>, ContextError> {
    let Some(raw) = std::env::var_os("ACCELERATOR_LINEAR_API_URL") else {
        return Ok(None);
    };
    let raw = raw.to_string_lossy();
    if !url_is_allowed(&raw, cfg!(feature = "test-loopback")) {
        return Err(ContextError::BadApiUrl(raw.into_owned()));
    }
    let url = Url::parse(&raw)
        .map_err(|_| ContextError::BadApiUrl(raw.into_owned()))?;
    Ok(Some(url))
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

/// Builds a Linear client from the working directory's configuration, honouring
/// the validated base-URL seam, and returns it alongside the integrations root.
///
/// # Errors
///
/// [`ContextError`] for a bad override, an unreadable config, or a client that
/// cannot be constructed.
pub fn build_client() -> Result<(LinearClient, PathBuf), ContextError> {
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
        Some(endpoint) => {
            build_with_override(&context, &integrations_root, endpoint)
                .map_err(ContextError::Client)?
        }
        None => LinearClient::from_config(&context, &integrations_root)
            .map_err(ContextError::Client)?,
    };
    Ok((client, integrations_root))
}

/// The override branch: the whole client `from_config` builds, differing only in
/// the GraphQL endpoint and the loopback-admitting upload transport.
fn build_with_override(
    context: &CredentialContext<'_>,
    integrations_root: &Path,
    endpoint: Url,
) -> Result<LinearClient, ClientError> {
    let credentials = resolve_credentials(context, integrations_root)?;
    let transport = Transport::new(
        endpoint,
        credentials,
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )?;
    let allow_loopback = cfg!(feature = "test-loopback");
    let upload = UploadTransport::new(allow_loopback, Duration::from_secs(1))?;
    let team_key = catalogue_team_key(integrations_root);
    let states = CatalogueStates::load(integrations_root);
    Ok(LinearClient::new(
        transport,
        upload,
        team_key,
        Box::new(states),
    ))
}
