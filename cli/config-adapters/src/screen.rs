//! The personal level as composition found it: an ignored `config.local.md`
//! answers as absent and is never opened.

use std::path::PathBuf;

use config::{
    ConfigError, Level, Node, PersonalFile, ReadConfigLevel, ReadContent,
};

use crate::store::FileConfigStore;

/// The store's read ports, behind the fact `compose` established about the
/// personal file. A file that turns insecure after that probe still fails
/// closed, because the store checks the file on every read.
#[derive(Clone)]
pub struct ScreenedStore {
    store: FileConfigStore,
    personal_file: PersonalFile,
}

impl ScreenedStore {
    pub(crate) const fn new(
        store: FileConfigStore,
        personal_file: PersonalFile,
    ) -> Self {
        Self {
            store,
            personal_file,
        }
    }

    #[must_use]
    pub fn with_plugin_root(self, plugin_root: Option<PathBuf>) -> Self {
        Self {
            store: self.store.with_plugin_root(plugin_root),
            ..self
        }
    }

    #[must_use]
    pub const fn personal_file(&self) -> &PersonalFile {
        &self.personal_file
    }

    fn ignores(&self, level: Level) -> bool {
        level == Level::Personal
            && matches!(self.personal_file, PersonalFile::Ignored { .. })
    }
}

impl ReadConfigLevel for ScreenedStore {
    fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
        if self.ignores(level) {
            return Ok(None);
        }
        self.store.read(level)
    }
}

impl ReadContent for ScreenedStore {
    fn config_body(&self, level: Level) -> Result<Option<String>, ConfigError> {
        if self.ignores(level) {
            return Ok(None);
        }
        self.store.config_body(level)
    }

    fn skill_context(
        &self,
        skill: &str,
    ) -> Result<Option<String>, ConfigError> {
        self.store.skill_context(skill)
    }

    fn skill_instructions(
        &self,
        skill: &str,
    ) -> Result<Option<String>, ConfigError> {
        self.store.skill_instructions(skill)
    }
}
