//! Everything retiring an ID and promoting a draft read and write, owned in
//! one place so every command that changes an item's identity assembles the
//! same ports.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use corpus::store::AtomicWrite;
use corpus_adapters::FileRecoveryCopies;
use corpus_adapters::LockdirLock;
use corpus_adapters::RealFs;
use vcs_adapters::library::InProcessProbe;
use work_adapters::promotion_records::FilePromotionRecords;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::create::CreationStore;

pub struct IdentityWorkspace {
    work_dir: PathBuf,
    state_dir: PathBuf,
    integrations_root: PathBuf,
    integration: String,
    roots: Vec<PathBuf>,
    locks: LockdirLock,
    recovery: FileRecoveryCopies,
    status: VcsWorkingCopyStatus,
}

impl IdentityWorkspace {
    /// Opens the workspace of the repository at `repo_root`, creating the
    /// state directories its records and baseline are written under.
    ///
    /// # Errors
    ///
    /// A message naming the directory that could not be created, or the
    /// configuration that could not be read.
    pub fn open(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        work_dir: &Path,
        integrations_root: &Path,
        integration: &str,
    ) -> Result<Self, String> {
        let state_dir = repo_root.join(crate::sync::STATE_DIR);
        // On a never-synced integration the baseline's directory does not
        // exist yet, and the atomic-write containment check canonicalises it
        // as its trusted root, so the first baseline write fails unless it
        // is present.
        let baseline_dir = baseline::path(integrations_root, integration)
            .parent()
            .map_or_else(|| integrations_root.to_path_buf(), Path::to_path_buf);
        for dir in [&state_dir, &baseline_dir] {
            std::fs::create_dir_all(dir).map_err(|error| {
                format!("could not create {}: {error}", dir.display())
            })?;
        }
        let roots = crate::sync::corpus_roots(config, repo_root)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            work_dir: work_dir.to_path_buf(),
            locks: LockdirLock::new(work_dir),
            recovery: FileRecoveryCopies::new(&state_dir, repo_root),
            status: VcsWorkingCopyStatus::probed_from(
                repo_root,
                &InProcessProbe,
            ),
            state_dir,
            integrations_root: integrations_root.to_path_buf(),
            integration: integration.to_owned(),
            roots,
        })
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub const fn recovery(&self) -> &FileRecoveryCopies {
        &self.recovery
    }

    pub const fn status(&self) -> &VcsWorkingCopyStatus {
        &self.status
    }

    pub fn baseline_path(&self) -> PathBuf {
        baseline::path(&self.integrations_root, &self.integration)
    }

    pub fn baseline<'a>(
        &self,
        writer: &'a dyn AtomicWrite,
    ) -> BaselineStore<'a> {
        BaselineStore::new(self.baseline_path(), &RealFs, writer)
    }

    pub fn retirement_ports<'a>(
        &'a self,
        store: &'a dyn CreationStore,
        baseline: &'a BaselineStore<'a>,
    ) -> RetirementPorts<'a> {
        RetirementPorts {
            files: RetirementFiles {
                reader: &RealFs,
                writer: store,
                creator: store,
                remover: store,
                file_locks: &self.locks,
                recovery: &self.recovery,
            },
            layout: CorpusLayout {
                roots: &self.roots,
                work_dir: &self.work_dir,
            },
            walker: &RealFs,
            status: &self.status,
            lock: &self.locks,
            baseline,
        }
    }

    pub fn promotion_records<'a>(
        &self,
        writer: &'a dyn AtomicWrite,
    ) -> FilePromotionRecords<'a> {
        FilePromotionRecords::new(
            &self.integrations_root,
            &self.integration,
            writer,
        )
    }

    pub fn retirement_records<'a>(
        &self,
        writer: &'a dyn AtomicWrite,
    ) -> FileRetirementRecords<'a> {
        FileRetirementRecords::new(&self.state_dir, writer)
    }
}
