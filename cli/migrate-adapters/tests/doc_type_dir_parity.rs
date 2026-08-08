//! A shape change to `config::paths::DocTypeDir` that isn't mirrored in
//! `FileMigrationContext`'s conversion should fail this test, not silently
//! drift.

use config::ConfigService;
use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;
use migrate::ports::MigrationContext;
use migrate_adapters::context::FileMigrationContext;
use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

#[test]
fn every_doc_type_dir_matches_configs_own_resolution_field_for_field(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    std::fs::create_dir_all(dir.path().join(".git"))?;

    let store =
        FileConfigStore::at(dir.path()).with_legacy_policy(LegacyPolicy::Allow);
    let service = ConfigService::new(store.clone(), store);
    let expected = config::paths::doc_type_dirs(&service)?;

    let ctx = FileMigrationContext::new(dir.path());
    let actual = ctx.doc_type_dirs();

    assert_eq!(actual.len(), expected.len());
    for resolved in &expected {
        let found = actual
            .iter()
            .find(|candidate| candidate.doc_type == resolved.doc_type);
        assert!(
            found.is_some(),
            "no migrate::DocTypeDir for {}",
            resolved.doc_type
        );
        if let Some(found) = found {
            assert_eq!(found.dir, dir.path().join(&resolved.dir));
        }
    }
    Ok(())
}
