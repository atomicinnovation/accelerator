use std::path::Path;
use std::path::PathBuf;

use config::ConfigError;
use config_adapters::Composed;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus::LinkageIds;
use corpus_adapters::RealFs;
use corpus_adapters::YamlFrontmatter;
use migrate::ports::MigrationError;
use migrate::ports::SyncBaselines;
use migrate::ports::WorkItemIdentity;
use migrate_adapters::context::Capabilities;
use work::id_pattern::IdKey;
use work::id_pattern::IdPattern;
use work::id_scheme::WorkItemIdScheme;
use work::legacy_ids::LegacyNumericIds;

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
        let pattern =
            IdPattern::parse(pattern).map_err(|error| failure(&error))?;
        let key = key
            .filter(|_| pattern.references_key())
            .map(IdKey::parse)
            .transpose()
            .map_err(|error| failure(&error))?;
        WorkItemIdScheme::new(pattern, key)
            .canonicalise(raw)
            .map_err(|error| failure(&error))
    }

    fn references_key(&self, pattern: &str) -> bool {
        work::legacy_ids::references_key(pattern)
    }

    fn legacy_linkage(&self) -> &dyn LinkageIds {
        &LegacyNumericIds
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
