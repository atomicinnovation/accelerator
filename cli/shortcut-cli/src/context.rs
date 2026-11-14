//! The credential context, the validated base-URL seam, and the two things
//! built from them: a bare session for `init`, which runs before any
//! catalogue exists, and a full client for `show` and `search`.
//!
//! `ACCELERATOR_SHORTCUT_API_URL` ships in the release binary, but only an
//! https origin on Shortcut's API host is admitted. A loopback origin is
//! admitted only under the test-only `test-loopback` feature. A value that is
//! set but not admitted is a usage error, never a fall-through to production.

use std::path::Path;
use std::path::PathBuf;

use config::consent::CommandPolicy;
use config::consent::Notice;
use config::consent::Refusal;
use config::credentials::CredentialContext;
use config::ConfigAccess;
use config::Key;
use config_adapters::compose;
use config_adapters::credentials::project_credential_context;
use config_adapters::Composed;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use shortcut_client::auth::resolve_credentials;
use shortcut_client::paths::url_is_allowed;
use shortcut_client::paths::PRODUCTION_BASE;
use shortcut_client::transport::Transport;
use shortcut_client::ClientError;
use shortcut_client::ShortcutClient;
use shortcut_client::Url;
use tracker_support::ClockJitter;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

pub enum ContextError {
    BadApiUrl(String),
    Config(String),
    Client(ClientError),
}

/// Whether a command writes, to the project tree or to Shortcut. A writer
/// refuses to run beside an ignored personal config, whose keys would
/// silently come from the team file instead.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Read,
    Write,
}

pub struct Session {
    pub transport: Transport,
    pub base: Url,
    pub refusals: Vec<Refusal>,
    pub notice: Option<Notice>,
    pub config: Composed,
    pub integrations_root: PathBuf,
    pub project_root: PathBuf,
}

fn api_base() -> Result<Url, ContextError> {
    let Some(raw) = std::env::var_os("ACCELERATOR_SHORTCUT_API_URL") else {
        return Url::parse(PRODUCTION_BASE)
            .map_err(|error| ContextError::Config(error.to_string()));
    };
    let raw = raw.to_string_lossy();
    if !url_is_allowed(&raw, cfg!(feature = "test-loopback")) {
        return Err(ContextError::BadApiUrl(raw.into_owned()));
    }
    Url::parse(&raw).map_err(|_| ContextError::BadApiUrl(raw.into_owned()))
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

fn transport_config(
    config: &dyn ConfigAccess,
) -> Result<TransportConfig, ContextError> {
    let ceilings = tracker_support::pull::resolve_ceilings(config, "shortcut")
        .map_err(ContextError::Config)?;
    Ok(TransportConfig {
        discovery_max_pages: ceilings.discovery_pages,
        keyed_read_max_pages: ceilings.keyed_read_pages,
        ..TransportConfig::default()
    })
}

struct Ground {
    base: Url,
    composed: Composed,
    root: PathBuf,
    start: PathBuf,
    integrations_root: PathBuf,
    transport_config: TransportConfig,
}

fn ground(intent: Intent) -> Result<Ground, ContextError> {
    let base = api_base()?;
    let start = std::env::current_dir().map_err(|error| {
        ContextError::Config(format!(
            "could not read the working directory: {error}"
        ))
    })?;
    let composed = compose(&start, LegacyPolicy::Reject)
        .map_err(|error| ContextError::Config(error.to_string()))?;
    composed.report_ignored_personal_file();
    if intent == Intent::Write {
        composed
            .require_readable_personal_file()
            .map_err(|error| ContextError::Config(error.to_string()))?;
    }
    let root = FileConfigStore::discover_root(&start);
    let integrations_root = integrations_dir(&composed.service, &root)?;
    let transport_config = transport_config(&composed.service)?;
    Ok(Ground {
        base,
        composed,
        root,
        start,
        integrations_root,
        transport_config,
    })
}

fn with_credentials<T>(
    ground: &Ground,
    build: impl FnOnce(&CredentialContext<'_>) -> Result<T, ClientError>,
) -> Result<T, ContextError> {
    let ports = config_adapters::credential_ports(
        vcs_adapters::InProcessTracking,
        &ground.root,
        &ground.start,
    );
    let context = project_credential_context(
        &ground.root,
        &ports,
        &ground.composed.service,
        CommandPolicy::DEFAULT_TIMEOUT,
    );
    build(&context).map_err(ContextError::Client)
}

/// # Errors
///
/// [`ContextError`] for a refused override, an unreadable config, or a token
/// that cannot be resolved.
pub fn open_session(intent: Intent) -> Result<Session, ContextError> {
    let ground = ground(intent)?;
    let transport_config = ground.transport_config;
    let (transport, refusals, notice) = with_credentials(&ground, |context| {
        let credentials = resolve_credentials(context)?;
        let transport = Transport::new(
            &credentials,
            transport_config,
            Box::new(SystemSleeper),
            Box::new(ClockJitter),
        )?;
        Ok((transport, credentials.refusals, credentials.notice))
    })?;
    Ok(Session {
        transport,
        base: ground.base,
        refusals,
        notice,
        config: ground.composed,
        integrations_root: ground.integrations_root,
        project_root: ground.root,
    })
}

/// # Errors
///
/// As [`open_session`], and [`ClientError::NoCatalogue`] when there is no
/// catalogue to read.
pub fn build_client(intent: Intent) -> Result<ShortcutClient, ContextError> {
    let ground = ground(intent)?;
    with_credentials(&ground, |context| {
        ShortcutClient::from_config_at(
            ground.base.clone(),
            context,
            &ground.integrations_root,
            ground.transport_config,
        )
    })
}
