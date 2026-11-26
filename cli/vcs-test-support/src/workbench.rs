//! A test's own directory and hermetic environment.
//!
//! The workbench is the one place a test acquires a fixture: by adopting the
//! run's shared build of a family, by copying part of it to write in, or by
//! building it privately when the run hands none over.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::copies::copy_part;
use crate::family::BuiltFixtures;
use crate::family::CopyableFamily;
use crate::family::PrivateBuild;
use crate::family::SharedFamily;
use crate::fixture_log::Event;
use crate::fixtures::Matrix;
use crate::handover::Handover;
use crate::handover::SHARED_WORKBENCHES_VARIABLE;
use crate::hermetic::assert_no_repository_ancestor;
use crate::hermetic::Hermetic;
use crate::stalled_migration::MigrateBinary;
use crate::stalled_migration::StallKey;
use crate::stalled_migration::StalledMigrations;
use crate::status_log::States;
use crate::status_log::StatusLogPart;
use crate::Error;

/// The states of one private set, keyed by name.
pub type PrivateStates = BTreeMap<&'static str, PathBuf>;

/// Builds one private set beneath a directory with a hermetic environment.
pub type PrivateSetBuilder =
    fn(&Path, &Hermetic) -> Result<PrivateStates, Error>;

#[derive(Debug, Clone, Copy)]
enum Acquisition {
    Adopted,
    Copied,
    Built,
}

impl fmt::Display for Acquisition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Adopted => "adopted",
            Self::Copied => "copied",
            Self::Built => "built",
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Holding {
    fixtures: BuiltFixtures,
    acquisition: Acquisition,
}

pub struct Workbench {
    directory: TempDir,
    path: PathBuf,
    hermetic: Hermetic,
    handover: Handover,
    holds: Cell<Option<Holding>>,
}

/// A fixture the workbench handed out. A private build lives inside the
/// workbench, so the fixture cannot outlive it.
pub struct Held<'w, F> {
    fixture: F,
    workbench: PhantomData<&'w Workbench>,
}

impl<F> Deref for Held<'_, F> {
    type Target = F;

    fn deref(&self) -> &F {
        &self.fixture
    }
}

impl Workbench {
    /// Opens a workbench with what the run handed this test.
    ///
    /// # Errors
    ///
    /// When the workbench cannot be created.
    pub fn for_this_test() -> Result<Self, Error> {
        Self::with(Handover::from_environment())
    }

    /// Opens a workbench with an explicit hand-over.
    ///
    /// Tests use [`Self::for_this_test`]: a hand-over that did not come from
    /// the run hides the workbench's builds from the run's fixture log, so a
    /// guard permits this only in the files that test the workbench itself.
    ///
    /// # Errors
    ///
    /// When the workbench cannot be created, or lies inside a repository.
    pub fn with(handover: Handover) -> Result<Self, Error> {
        let builder = tempfile::Builder::new().prefix("vcs-workbench-").clone();
        let directory = match &handover.workbenches {
            Some(workbenches) => {
                builder.tempdir_in(workbenches).map_err(|error| {
                    Error::message(format!(
                        "{SHARED_WORKBENCHES_VARIABLE} names {}: {error}",
                        workbenches.display()
                    ))
                })?
            }
            None => builder.tempdir_in(env::temp_dir())?,
        };
        let path = directory.path().canonicalize()?;
        assert_no_repository_ancestor(&path)?;
        let hermetic = Hermetic::rooted_at(&path)?;
        Ok(Self {
            directory,
            path,
            hermetic,
            handover,
            holds: Cell::new(None),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn hermetic(&self) -> &Hermetic {
        &self.hermetic
    }

    /// Adopts the family's shared tree when the run handed one over, and
    /// otherwise builds the family privately in this workbench.
    ///
    /// Treat the fixture as read-only: an adopted tree is shared by every
    /// test in the run, so write scratch files under [`Self::path`] instead.
    ///
    /// # Errors
    ///
    /// When the workbench already holds a fixture, the handed tree is missing
    /// or cannot be adopted, or the private build fails.
    pub fn shared<F: SharedFamily<Inputs = ()>>(
        &self,
    ) -> Result<Held<'_, F>, Error> {
        let fixtures = BuiltFixtures::Shared(F::FAMILY);
        self.hand_out(fixtures, || {
            if let Some(tree) = self.handover.shared_tree(F::FAMILY) {
                let fixture = adopt::<F>(tree)?;
                self.handover.log.record(&Event::Adoption {
                    family: F::FAMILY,
                    test: &self.handover.test,
                    hermetic_root: self.path(),
                })?;
                Ok((fixture, Acquisition::Adopted))
            } else {
                self.build_privately(fixtures, |directory, hermetic| {
                    F::build_privately(
                        directory,
                        hermetic,
                        &(),
                        PrivateBuild(()),
                    )
                })
            }
        })
    }

    /// Copies `part` of the family's shared tree into this workbench when the
    /// run handed one over, and otherwise builds `part` privately here.
    ///
    /// The fixture is the test's own to change.
    ///
    /// # Errors
    ///
    /// When the workbench already holds a fixture, the handed tree is missing
    /// or cannot be adopted or copied, a copied file still names the shared
    /// tree, or the private build fails.
    pub fn writable<F: CopyableFamily + SharedFamily<Inputs = ()>>(
        &self,
        part: F::Part,
    ) -> Result<Held<'_, F>, Error> {
        self.writable_with::<F>(part, &())
    }

    /// As [`Self::writable`], for a family whose private build needs
    /// `inputs`. A copy of the handed tree leaves them unused.
    ///
    /// # Errors
    ///
    /// As [`Self::writable`].
    pub fn writable_with<F: CopyableFamily>(
        &self,
        part: F::Part,
        inputs: &F::Inputs,
    ) -> Result<Held<'_, F>, Error> {
        let fixtures = BuiltFixtures::Shared(F::FAMILY);
        self.hand_out(fixtures, || {
            if let Some(tree) = self.handover.shared_tree(F::FAMILY) {
                Ok((self.copied::<F>(tree, &part)?, Acquisition::Copied))
            } else {
                self.build_privately(fixtures, |directory, hermetic| {
                    F::build_part_privately(
                        part,
                        directory,
                        hermetic,
                        inputs,
                        PrivateBuild(()),
                    )
                })
            }
        })
    }

    /// Builds one of the private sets, which are never shared, in this
    /// workbench.
    ///
    /// # Errors
    ///
    /// When the workbench already holds a fixture, or the build fails.
    pub fn private_states(
        &self,
        build: PrivateSetBuilder,
    ) -> Result<Held<'_, PrivateStates>, Error> {
        let fixtures = BuiltFixtures::PrivateSets;
        self.hand_out(fixtures, || self.build_privately(fixtures, build))
    }

    /// The run's matrix, shared with every other test: treat it as read-only
    /// and write scratch files under [`Self::path`].
    ///
    /// # Errors
    ///
    /// As [`Self::shared`].
    pub fn matrix(&self) -> Result<Held<'_, Matrix>, Error> {
        self.shared::<Matrix>()
    }

    /// The run's status-log set, shared with every other test: treat it as
    /// read-only and write scratch files under [`Self::path`].
    ///
    /// # Errors
    ///
    /// As [`Self::shared`].
    pub fn status_log_states(&self) -> Result<Held<'_, States>, Error> {
        self.shared::<States>()
    }

    /// A copy of `part` of the run's status-log set, the test's own to
    /// change.
    ///
    /// # Errors
    ///
    /// As [`Self::writable`].
    pub fn writable_status_log_states(
        &self,
        part: StatusLogPart,
    ) -> Result<Held<'_, States>, Error> {
        self.writable::<States>(part)
    }

    /// A copy of the stall keyed `key` from the run's stalled-migration
    /// set, the test's own to change.
    ///
    /// # Errors
    ///
    /// As [`Self::writable`].
    pub fn writable_stalled_migrations(
        &self,
        key: StallKey,
        migrate: &MigrateBinary,
    ) -> Result<Held<'_, StalledMigrations>, Error> {
        self.writable_with::<StalledMigrations>(key, migrate)
    }

    fn hand_out<F>(
        &self,
        fixtures: BuiltFixtures,
        acquire: impl FnOnce() -> Result<(F, Acquisition), Error>,
    ) -> Result<Held<'_, F>, Error> {
        self.refuse_a_second(fixtures)?;
        let (fixture, acquisition) = acquire()?;
        self.holds.set(Some(Holding {
            fixtures,
            acquisition,
        }));
        Ok(Held {
            fixture,
            workbench: PhantomData,
        })
    }

    fn build_privately<F>(
        &self,
        fixtures: BuiltFixtures,
        build: impl FnOnce(&Path, &Hermetic) -> Result<F, Error>,
    ) -> Result<(F, Acquisition), Error> {
        let fixture = build(self.path(), &self.hermetic)?;
        self.handover.log.record(&Event::PrivateBuild {
            fixtures,
            test: &self.handover.test,
            hermetic_root: self.path(),
        })?;
        Ok((fixture, Acquisition::Built))
    }

    fn copied<F: CopyableFamily>(
        &self,
        tree: &Path,
        part: &F::Part,
    ) -> Result<F, Error> {
        let shared = adopt::<F>(tree)?;
        let entries = F::entries(part, tree)?;
        let names: Vec<&str> = entries.iter().map(String::as_str).collect();
        let copy = copy_part(tree, &names, self.path())?;
        self.handover.log.record(&Event::Copy {
            family: F::FAMILY,
            test: &self.handover.test,
            hermetic_root: self.path(),
            copy: &copy.root,
            named_by: &copy.named_by,
        })?;
        copy.require_clean(
            format_args!("the copy of {}", F::FAMILY),
            format_args!("the shared tree {}", tree.display()),
        )?;
        Ok(shared.rebased(tree, self.path(), &entries))
    }

    fn refuse_a_second(&self, requested: BuiltFixtures) -> Result<(), Error> {
        match self.holds.get() {
            None => Ok(()),
            Some(Holding {
                fixtures,
                acquisition,
            }) => Err(Error::message(format!(
                "this workbench already holds {fixtures} ({acquisition}); \
                 open another workbench for {requested}"
            ))),
        }
    }
}

impl fmt::Debug for Workbench {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Workbench")
            .field("directory", &self.directory)
            .field("holds", &self.holds.get())
            .finish_non_exhaustive()
    }
}

fn adopt<F: SharedFamily>(tree: &Path) -> Result<F, Error> {
    let variable = F::FAMILY.handover_variable();
    if !tree.is_dir() {
        return Err(Error::message(format!(
            "{variable} hands over {}, which does not exist",
            tree.display()
        )));
    }
    F::adopt(tree).map_err(|error| {
        Error::message(format!(
            "{variable} hands over {}, which cannot be adopted: {error}",
            tree.display()
        ))
    })
}
