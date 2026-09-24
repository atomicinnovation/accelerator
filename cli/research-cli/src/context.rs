//! The project a call runs in: its root and its composed configuration.

use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess;
use config::ConfigError;
use config_adapters::compose;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;

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
