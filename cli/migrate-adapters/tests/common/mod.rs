use std::path::Path;
use std::path::PathBuf;

use config::ConfigError;
use config_adapters::Composed;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use corpus_adapters::PatternCanonicaliser;
use corpus_adapters::RealFs;
use corpus_adapters::YamlFrontmatter;
use migrate::ports::MigrationError;
use migrate::ports::SyncBaselines;
use migrate_adapters::context::Capabilities;

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
            canonicaliser: &PatternCanonicaliser,
            sync_baselines: &NoBaselines,
        }
    }
}
