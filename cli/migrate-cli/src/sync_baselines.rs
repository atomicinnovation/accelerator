//! Migration 0008's sync-baseline realignment, answered by the work context.

use std::path::Path;
use std::path::PathBuf;

use corpus::store::AtomicWrite;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use migrate::ports::MigrationError;
use migrate::ports::SyncBaselines;
use work_adapters::sync::realign::realign_baselines;

pub struct WorkSyncBaselines;

fn store_rooted_at(dir: &Path) -> Box<dyn AtomicWrite> {
    Box::new(FileCorpusStore::new(dir))
}

impl SyncBaselines for WorkSyncBaselines {
    fn realign(
        &self,
        integrations_root: &Path,
        pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        realign_baselines(
            integrations_root,
            pre_migration,
            &RealFs,
            &store_rooted_at,
        )
        .map_err(|error| MigrationError::new(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    use migrate::ports::SyncBaselines as _;
    use tempfile::TempDir;

    use super::WorkSyncBaselines;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn an_unreadable_baseline_fails_with_its_store_error_text(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let baseline = dir.path().join("linear/last-sync.json");
        fs::create_dir_all(dir.path().join("linear"))?;
        fs::write(&baseline, "{}")?;
        fs::set_permissions(&baseline, fs::Permissions::from_mode(0o000))?;
        let Some(denied) = fs::read(&baseline).err() else {
            return Err("this user can read a 0o000 file".into());
        };

        let result = WorkSyncBaselines.realign(dir.path(), &[]);

        let shown = baseline.display();
        assert_eq!(
            result.err().map(|error| error.to_string()),
            Some(format!("I/O error on '{shown}': reading {shown}: {denied}"))
        );
        Ok(())
    }

    #[test]
    fn a_malformed_baseline_realigns_nothing_and_is_left_alone(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let baseline = dir.path().join("linear/last-sync.json");
        fs::create_dir_all(dir.path().join("linear"))?;
        fs::write(&baseline, "<<<<<<< HEAD\n")?;

        let realigned = WorkSyncBaselines.realign(dir.path(), &[])?;

        assert_eq!(realigned, 0);
        assert_eq!(fs::read_to_string(&baseline)?, "<<<<<<< HEAD\n");
        Ok(())
    }

    #[test]
    fn an_unlistable_integrations_root_fails_with_the_io_text(
    ) -> Result<(), TestError> {
        let dir = TempDir::new()?;
        let not_a_directory = dir.path().join("integrations");
        fs::write(&not_a_directory, "")?;
        let expected = fs::read_dir(&not_a_directory).err();

        let result = WorkSyncBaselines.realign(&not_a_directory, &[]);

        assert_eq!(
            result.err().map(|error| error.to_string()),
            expected.map(|error| error.to_string())
        );
        Ok(())
    }
}
