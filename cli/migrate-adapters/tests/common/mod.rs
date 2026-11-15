use std::path::Path;
use std::path::PathBuf;

use config::ConfigError;
use config_adapters::Composed;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus_adapters::RealFs;
use corpus_adapters::YamlFrontmatter;
use migrate::ports::MigrationError;
use migrate::ports::SyncBaselines;
use migrate::ports::WorkItemIdentity;
use migrate_adapters::context::Capabilities;
use work_adapters::config::scheme_from_settings;
use work_adapters::config::KeySource;

pub struct NoBaselines;

impl SyncBaselines for NoBaselines {
    fn realign(
        &self,
        _integrations_root: &Path,
        _pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        Ok(0)
    }
}

pub struct WorkIdentity;

fn failure(error: &impl ToString) -> MigrationError {
    MigrationError::new(error.to_string())
}

impl WorkItemIdentity for WorkIdentity {
    fn canonicalise(
        &self,
        raw: &str,
        pattern: &str,
        key: Option<&str>,
    ) -> Result<String, MigrationError> {
        scheme_from_settings(pattern, key, KeySource::LegacyDefaultProjectCode)
            .map_err(|error| failure(&error))?
            .canonicalise(raw)
            .map_err(|error| failure(&error))
    }
}

/// The production capabilities over a repository root, with sync-baseline
/// realignment stubbed out.
pub struct Composition {
    composed: Composed,
}

impl Composition {
    pub fn at(root: &Path) -> Result<Self, ConfigError> {
        Ok(Self {
            composed: Composed::over(
                FileConfigStore::at(root)
                    .with_legacy_policy(LegacyPolicy::Allow),
            )?,
        })
    }

    pub fn capabilities(&self) -> Capabilities<'_> {
        Capabilities {
            config: &self.composed.service,
            walker: &RealFs,
            reader: &RealFs,
            frontmatter: &YamlFrontmatter,
            identity: &WorkIdentity,
            sync_baselines: &NoBaselines,
            version_control: None,
        }
    }
}
