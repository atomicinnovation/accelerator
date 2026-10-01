//! The project a call runs in: its root and its composed configuration.

use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess;
use config::ConfigError;
use config::Key;
use config_adapters::compose;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus::DocTypeKey;

pub struct ProjectContext {
    pub root: PathBuf,
    pub config: Box<dyn ConfigAccess>,
}

impl ProjectContext {
    /// The project enclosing `cwd`.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when the project's configuration cannot be composed.
    pub fn enclosing(cwd: &Path) -> Result<Self, ConfigError> {
        let composed = compose(cwd, LegacyPolicy::Reject)?;
        Ok(Self {
            root: FileConfigStore::discover_root(cwd),
            config: Box::new(composed.service),
        })
    }
}

impl ProjectContext {
    /// Where research keeps state across calls: `research/` under the
    /// configured `paths.tmp`.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when `paths.tmp` cannot be resolved.
    pub fn research_scratch(&self) -> Result<PathBuf, ConfigError> {
        let key = Key::parse("paths.tmp")?;
        let tmp = self.config.effective_nonempty(&key, None)?.rendered();
        Ok(self.root.join(tmp).join("research"))
    }
}

impl ProjectContext {
    /// A document type's configured directory, relative paths resolved
    /// against the project root.
    ///
    /// # Errors
    ///
    /// A message when the type has no configured directory or its key cannot
    /// be resolved.
    pub fn type_dir(&self, key: DocTypeKey) -> Result<PathBuf, String> {
        let path_key = key.config_path_key().ok_or_else(|| {
            format!(
                "document type '{}' has no configured directory",
                key.wire_str()
            )
        })?;
        let raw = config::paths::resolve_with_fallback(
            self.config.as_ref(),
            path_key,
            None,
        )
        .map_err(|error| error.to_string())?;
        Ok(self.root.join(raw))
    }
}
