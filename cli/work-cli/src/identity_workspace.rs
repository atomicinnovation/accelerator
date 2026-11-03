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
use vcs_adapters::InProcessTracking;
use work_adapters::promotion_records::FilePromotionRecords;
use work_adapters::retirement::CorpusLayout;
use work_adapters::retirement::RetirementFiles;
use work_adapters::retirement::RetirementPorts;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::sync::baseline;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

use crate::bounded_roots::RootRefusal;
use crate::bounded_roots::ValidatedRoots;
use crate::bounded_roots::ValidatedStateRoots;
use crate::create::CreationStore;

struct Directories<'a> {
    work: &'a Path,
    integrations: &'a Path,
    state: &'a Path,
}

/// The corpus a retirement rewrites and the state it keeps beside it,
/// whether or not an integration is configured.
pub struct CorpusWorkspace {
    work_dir: PathBuf,
    state_dir: PathBuf,
    roots: Vec<PathBuf>,
    locks: LockdirLock,
    recovery: FileRecoveryCopies,
    status: VcsWorkingCopyStatus,
    tracking: InProcessTracking,
}

impl CorpusWorkspace {
    /// Opens the corpus of the repository at `repo_root`, creating nothing.
    ///
    /// # Errors
    ///
    /// A message naming the configuration that could not be read.
    pub fn open(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        work_dir: &Path,
    ) -> Result<Self, String> {
        Self::under(
            config,
            repo_root,
            work_dir,
            repo_root.join(crate::sync::STATE_DIR),
        )
    }

    fn under(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        work_dir: &Path,
        state_dir: PathBuf,
    ) -> Result<Self, String> {
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
            tracking: InProcessTracking,
            state_dir,
            roots,
        })
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub const fn locks(&self) -> &LockdirLock {
        &self.locks
    }

    pub const fn recovery(&self) -> &FileRecoveryCopies {
        &self.recovery
    }

    pub const fn status(&self) -> &VcsWorkingCopyStatus {
        &self.status
    }

    pub const fn tracking(&self) -> &InProcessTracking {
        &self.tracking
    }

    pub fn retirement_ports<'a>(
        &'a self,
        store: &'a dyn CreationStore,
        baseline: Option<&'a BaselineStore<'a>>,
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

    pub fn retirement_records<'a>(
        &self,
        writer: &'a dyn AtomicWrite,
    ) -> FileRetirementRecords<'a> {
        FileRetirementRecords::new(&self.state_dir, writer)
    }
}

pub struct IdentityWorkspace {
    corpus: CorpusWorkspace,
    integrations_root: PathBuf,
    integration: String,
}

/// Why a workspace could not be opened: a configured root it refuses to
/// write under, or a directory or configuration it could not read.
#[derive(Debug)]
pub enum OpenFailure {
    Refused(RootRefusal),
    Failed(String),
}

impl IdentityWorkspace {
    /// Opens the workspace a sync run writes through, under roots already
    /// checked.
    ///
    /// # Errors
    ///
    /// A message naming the directory that could not be created, or the
    /// configuration that could not be read.
    pub fn for_sync(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        roots: &ValidatedRoots,
        integration: &str,
    ) -> Result<Self, String> {
        Self::under(
            config,
            repo_root,
            &Directories {
                work: roots.work_items.root(),
                integrations: roots.integrations.root(),
                state: roots.recovery_state.root(),
            },
            integration,
        )
    }

    /// Opens the workspace a retirement or promotion writes through, once
    /// the state root its recovery copies live under has been checked.
    ///
    /// # Errors
    ///
    /// [`OpenFailure::Refused`] naming the refused root, or
    /// [`OpenFailure::Failed`] naming the directory that could not be
    /// created or the configuration that could not be read.
    pub fn for_identity_change(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        work_dir: &Path,
        integrations_root: &Path,
        integration: &str,
    ) -> Result<(Self, ValidatedStateRoots), OpenFailure> {
        let roots = ValidatedStateRoots::check(repo_root)
            .map_err(OpenFailure::Refused)?;
        let workspace = Self::under(
            config,
            repo_root,
            &Directories {
                work: work_dir,
                integrations: integrations_root,
                state: roots.recovery_state.root(),
            },
            integration,
        )
        .map_err(OpenFailure::Failed)?;
        Ok((workspace, roots))
    }

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
        Self::under(
            config,
            repo_root,
            &Directories {
                work: work_dir,
                integrations: integrations_root,
                state: &repo_root.join(crate::sync::STATE_DIR),
            },
            integration,
        )
    }

    fn under(
        config: &dyn ConfigAccess,
        repo_root: &Path,
        directories: &Directories<'_>,
        integration: &str,
    ) -> Result<Self, String> {
        let corpus = CorpusWorkspace::under(
            config,
            repo_root,
            directories.work,
            directories.state.to_path_buf(),
        )?;
        // On a never-synced integration the baseline's directory does not
        // exist yet, and the atomic-write containment check canonicalises it
        // as its trusted root, so the first baseline write fails unless it
        // is present.
        let baseline_dir =
            baseline::path(directories.integrations, integration)
                .parent()
                .map_or_else(
                    || directories.integrations.to_path_buf(),
                    Path::to_path_buf,
                );
        for dir in [corpus.state_dir(), &baseline_dir] {
            std::fs::create_dir_all(dir).map_err(|error| {
                format!("could not create {}: {error}", dir.display())
            })?;
        }
        Ok(Self {
            corpus,
            integrations_root: directories.integrations.to_path_buf(),
            integration: integration.to_owned(),
        })
    }

    pub const fn corpus(&self) -> &CorpusWorkspace {
        &self.corpus
    }

    pub fn state_dir(&self) -> &Path {
        self.corpus.state_dir()
    }

    pub const fn locks(&self) -> &LockdirLock {
        self.corpus.locks()
    }

    pub const fn recovery(&self) -> &FileRecoveryCopies {
        self.corpus.recovery()
    }

    pub const fn status(&self) -> &VcsWorkingCopyStatus {
        self.corpus.status()
    }

    pub const fn tracking(&self) -> &InProcessTracking {
        self.corpus.tracking()
    }

    pub fn integrations_root(&self) -> &Path {
        &self.integrations_root
    }

    pub fn integration(&self) -> &str {
        &self.integration
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
        self.corpus.retirement_ports(store, Some(baseline))
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
        self.corpus.retirement_records(writer)
    }
}
