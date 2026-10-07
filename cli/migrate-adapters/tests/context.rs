//! `FileMigrationContext` over the production capabilities: the manifest it
//! records, and how it resolves the integrations root it hands to sync
//! baseline realignment.

mod common;

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use common::Composition;
use migrate::ports::ManifestStore as _;
use migrate::ports::MigrationContext as _;
use migrate::ports::MigrationError;
use migrate::ports::SyncBaselines;
use migrate_adapters::context::Capabilities;
use migrate_adapters::context::FileMigrationContext;
use migrate_adapters::manifest_store::FileManifestStore;
use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

#[derive(Default)]
struct RecordingBaselines {
    roots: RefCell<Vec<PathBuf>>,
}

impl SyncBaselines for RecordingBaselines {
    fn realign(
        &self,
        integrations_root: &Path,
        _pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        self.roots
            .borrow_mut()
            .push(integrations_root.to_path_buf());
        Ok(3)
    }
}

fn write_team_config(root: &Path, content: &str) -> Result<(), TestError> {
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(root.join(".accelerator/config.md"), content)?;
    Ok(())
}

#[test]
fn a_write_records_the_path_once_even_across_two_write_points(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let composition = Composition::at(dir.path())?;
    let ctx = FileMigrationContext::new(dir.path(), composition.capabilities());

    ctx.write(&dir.path().join("meta/work/a.md"), "one")?;
    ctx.write(&dir.path().join("meta/work/a.md"), "two")?;
    ctx.write(&dir.path().join("meta/work/b.md"), "three")?;

    let manifest = FileManifestStore::new(dir.path());
    let mut recorded = manifest.manifest()?.unwrap_or_default();
    recorded.sort();
    assert_eq!(
        recorded,
        vec!["meta/work/a.md".to_owned(), "meta/work/b.md".to_owned()]
    );
    Ok(())
}

#[test]
fn a_relative_integrations_root_is_resolved_against_the_project_root(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    fs::create_dir_all(dir.path().join(".accelerator/state/integrations"))?;
    let composition = Composition::at(dir.path())?;
    let baselines = RecordingBaselines::default();
    let ctx = FileMigrationContext::new(
        dir.path(),
        Capabilities {
            sync_baselines: &baselines,
            ..composition.capabilities()
        },
    );

    let realigned = ctx.realign_sync_baseline(&[])?;

    assert_eq!(realigned, 3);
    assert_eq!(
        *baselines.roots.borrow(),
        vec![dir.path().join(".accelerator/state/integrations")]
    );
    Ok(())
}

#[test]
fn an_absolute_integrations_root_is_used_as_configured() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let elsewhere = TempDir::new()?;
    write_team_config(
        dir.path(),
        &format!(
            "---\npaths:\n  integrations: {}\n---\n",
            elsewhere.path().display()
        ),
    )?;
    let composition = Composition::at(dir.path())?;
    let baselines = RecordingBaselines::default();
    let ctx = FileMigrationContext::new(
        dir.path(),
        Capabilities {
            sync_baselines: &baselines,
            ..composition.capabilities()
        },
    );

    ctx.realign_sync_baseline(&[])?;

    assert_eq!(*baselines.roots.borrow(), vec![elsewhere.path().to_owned()]);
    Ok(())
}

#[test]
fn an_absent_integrations_root_realigns_nothing() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let composition = Composition::at(dir.path())?;
    let baselines = RecordingBaselines::default();
    let ctx = FileMigrationContext::new(
        dir.path(),
        Capabilities {
            sync_baselines: &baselines,
            ..composition.capabilities()
        },
    );

    assert_eq!(ctx.realign_sync_baseline(&[])?, 0);
    assert!(baselines.roots.borrow().is_empty());
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn a_bare_number_is_canonicalised_under_the_configured_pattern(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    write_team_config(
        dir.path(),
        "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n  \
         default_project_code: ACME\n---\n",
    )?;
    let composition = Composition::at(dir.path())?;
    let ctx = FileMigrationContext::new(dir.path(), composition.capabilities());

    assert_eq!(ctx.canonicalise_work_item_id("7")?, "ACME-0007");
    Ok(())
}
