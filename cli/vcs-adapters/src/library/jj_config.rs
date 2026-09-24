//! jj's config stack, discovered and resolved as jj-cli does for `jj status`
//! but read-only: a missing per-id config is never generated and a legacy one
//! never migrated.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;

use etcetera::BaseStrategy as _;
use jj_lib::config::ConfigGetError;
use jj_lib::config::ConfigResolutionContext;
use jj_lib::config::ConfigSource;
use jj_lib::config::StackedConfig;
use jj_lib::settings::HumanByteSize;

/// Everything jj-cli reads from the process to find and resolve its config.
pub(super) struct JjConfigEnvironment {
    pub jj_config: Option<OsString>,
    pub home: Option<PathBuf>,
    pub user_config_dir: Option<PathBuf>,
    pub system_config_paths: Vec<PathBuf>,
    pub hostname: String,
    pub variables: HashMap<String, String>,
}

impl JjConfigEnvironment {
    pub(super) fn from_process() -> Self {
        Self {
            jj_config: std::env::var_os("JJ_CONFIG"),
            home: etcetera::home_dir()
                .ok()
                .map(|dir| dunce::canonicalize(&dir).unwrap_or(dir)),
            user_config_dir: etcetera::choose_base_strategy()
                .ok()
                .map(|strategy| strategy.config_dir()),
            system_config_paths: if cfg!(unix) {
                vec![
                    PathBuf::from("/etc/jj/config.toml"),
                    PathBuf::from("/etc/jj/conf.d"),
                ]
            } else {
                Vec::new()
            },
            hostname: whoami::hostname().unwrap_or_default(),
            variables: std::env::vars_os()
                .filter_map(|(key, value)| {
                    Some((key.into_string().ok()?, value.into_string().ok()?))
                })
                .collect(),
        }
    }

    pub(super) fn status_resolution_context<'a>(
        &'a self,
        workspace_root: &'a Path,
        repo_path: &'a Path,
    ) -> ConfigResolutionContext<'a> {
        ConfigResolutionContext {
            home_dir: self.home.as_deref(),
            repo_path: Some(repo_path),
            workspace_path: Some(workspace_root),
            command: Some("status"),
            hostname: &self.hostname,
            environment: &self.variables,
        }
    }

    fn system_paths(&self) -> Vec<PathBuf> {
        if self.jj_config.is_some() {
            return Vec::new();
        }
        self.system_config_paths.clone()
    }

    /// `$JJ_CONFIG`'s paths when set (used exclusively), else the legacy
    /// `~/.jjconfig.toml` (only when it exists, or when the platform config
    /// directory is unknown), the platform `config.toml` and, when it exists,
    /// the platform `conf.d`.
    fn user_paths(&self) -> Vec<PathBuf> {
        if let Some(paths) = &self.jj_config {
            return std::env::split_paths(paths)
                .filter(|path| !path.as_os_str().is_empty())
                .collect();
        }
        let home_config =
            self.home.as_ref().map(|dir| dir.join(".jjconfig.toml"));
        let platform_config = self
            .user_config_dir
            .as_ref()
            .map(|dir| dir.join("jj").join("config.toml"));
        let platform_conf_d = self
            .user_config_dir
            .as_ref()
            .map(|dir| dir.join("jj").join("conf.d"));

        let mut paths = Vec::new();
        match home_config {
            Some(path) if path.exists() || platform_config.is_none() => {
                paths.push(path);
            }
            Some(_) | None => {}
        }
        paths.extend(platform_config);
        paths.extend(platform_conf_d.filter(|path| path.exists()));
        paths
    }

    fn per_id_config_root(&self, kind: &str) -> Option<PathBuf> {
        self.user_config_dir
            .as_ref()
            .map(|dir| dir.join("jj").join(kind))
    }
}

/// The files each layer of jj's config stack is read from.
pub(super) struct JjConfigSources {
    pub system: Vec<PathBuf>,
    pub user: Vec<PathBuf>,
    pub repo: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
}

impl JjConfigSources {
    pub(super) fn for_workspace(
        workspace_root: &Path,
        repo_path: &Path,
        environment: &JjConfigEnvironment,
    ) -> Self {
        Self {
            repo: per_id_config_file(
                repo_path,
                "config-id",
                "config.toml",
                environment.per_id_config_root("repos").as_deref(),
            ),
            workspace: per_id_config_file(
                &workspace_root.join(".jj"),
                "workspace-config-id",
                "workspace-config.toml",
                environment.per_id_config_root("workspaces").as_deref(),
            ),
            ..Self::user_level(environment)
        }
    }

    pub(super) fn user_level(environment: &JjConfigEnvironment) -> Self {
        Self {
            system: environment.system_paths(),
            user: environment.user_paths(),
            repo: None,
            workspace: None,
        }
    }

    /// The layers loaded in precedence order, before any scope is resolved.
    ///
    /// # Errors
    ///
    /// When a layer file exists but cannot be read or parsed.
    pub(super) fn stacked(&self) -> Result<StackedConfig, ConfigLoadError> {
        let mut config = StackedConfig::empty();
        for path in &self.system {
            load_path(&mut config, ConfigSource::System, path)?;
        }
        for path in &self.user {
            load_path(&mut config, ConfigSource::User, path)?;
        }
        if let Some(path) = &self.repo {
            load_path(&mut config, ConfigSource::Repo, path)?;
        }
        if let Some(path) = &self.workspace {
            load_path(&mut config, ConfigSource::Workspace, path)?;
        }
        Ok(config)
    }
}

pub(super) type ConfigLoadError = jj_lib::config::ConfigLoadError;

/// Loads `path` as a directory of `*.toml` layers or a single file, matching
/// jj-cli's own dispatch. A path that does not exist is skipped, since the
/// candidate lists carry paths that may not exist yet.
fn load_path(
    config: &mut StackedConfig,
    source: ConfigSource,
    path: &Path,
) -> Result<(), ConfigLoadError> {
    if path.is_dir() {
        config.load_dir(source, path)
    } else if path.is_file() {
        config.load_file(source, path)
    } else {
        Ok(())
    }
}

/// The config file a per-repo or per-workspace config id points at, read
/// without jj-lib's `SecureConfig`, which generates and migrates files.
fn per_id_config_file(
    dir: &Path,
    id_file: &str,
    legacy_file: &str,
    root_config_dir: Option<&Path>,
) -> Option<PathBuf> {
    match fs::read_to_string(dir.join(id_file)) {
        Ok(id) if is_config_id(&id) => root_config_dir
            .map(|root| root.join(&id).join("config.toml"))
            .filter(|path| path.is_file()),
        Ok(_) => None,
        Err(_) => Some(dir.join(legacy_file)).filter(|path| path.is_file()),
    }
}

fn is_config_id(id: &str) -> bool {
    id.len() == 20 && id.chars().all(|character| character.is_ascii_hexdigit())
}

const MAX_NEW_FILE_SIZE: &str = "snapshot.max-new-file-size";

/// The size above which a snapshot refuses to start tracking a new file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MaxNewFileSize {
    Bytes(NonZeroU64),
    Unlimited,
}

impl MaxNewFileSize {
    const JJ_DEFAULT: u64 = 1024 * 1024;

    /// # Errors
    ///
    /// When a layer cannot be loaded, a scope cannot be resolved, or the
    /// value is not a size.
    pub(super) fn resolve(
        sources: &JjConfigSources,
        context: &ConfigResolutionContext<'_>,
    ) -> Result<Self, UnresolvableLimit> {
        let stacked = sources
            .stacked()
            .map_err(|error| UnresolvableLimit(error.to_string()))?;
        let resolved = jj_lib::config::resolve(&stacked, context)
            .map_err(|error| UnresolvableLimit(error.to_string()))?;
        let bytes = match resolved
            .get_value_with(MAX_NEW_FILE_SIZE, HumanByteSize::try_from)
        {
            Ok(HumanByteSize(bytes)) => bytes,
            Err(ConfigGetError::NotFound { .. }) => Self::JJ_DEFAULT,
            Err(error) => return Err(UnresolvableLimit(error.to_string())),
        };
        Ok(NonZeroU64::new(bytes).map_or(Self::Unlimited, Self::Bytes))
    }

    /// Resolves the limit, or warns and imposes none, so an unreadable config
    /// makes a snapshot report more changes rather than fail.
    pub(super) fn resolve_or_unlimited(
        sources: &JjConfigSources,
        context: &ConfigResolutionContext<'_>,
    ) -> Self {
        Self::resolve(sources, context).unwrap_or_else(|error| {
            tracing::warn!(%error, "checking every new file");
            Self::Unlimited
        })
    }

    pub(super) const fn as_snapshot_limit(self) -> u64 {
        match self {
            Self::Bytes(bytes) => bytes.get(),
            Self::Unlimited => u64::MAX,
        }
    }
}

#[derive(Debug)]
pub(super) struct UnresolvableLimit(String);

impl std::fmt::Display for UnresolvableLimit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "could not resolve {MAX_NEW_FILE_SIZE}: {}",
            self.0
        )
    }
}

impl std::error::Error for UnresolvableLimit {}

#[cfg(test)]
mod tests;
