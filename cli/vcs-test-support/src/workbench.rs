//! A test's own directory and hermetic environment.
//!
//! The workbench is the one place a test acquires a shared fixture family: by
//! adopting the run's shared build, or by building it privately when the run
//! hands none over.

use std::cell::Cell;
use std::env;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::family::Family;
use crate::family::Fixtures;
use crate::fixture_log::Event;
use crate::handover::Handover;
use crate::handover::SHARED_WORKBENCHES_VARIABLE;
use crate::hermetic::assert_no_repository_ancestor;
use crate::hermetic::Hermetic;
use crate::Error;

/// Proof that a private build goes through a workbench, which logs it.
///
/// Only this crate can make one, so code elsewhere can implement
/// [`SharedFamily`] but never call its `build_privately`.
pub struct PrivateBuild(());

pub trait SharedFamily: Sized {
    const FAMILY: Family;

    /// Takes over a tree the run's build step made, building nothing.
    ///
    /// # Errors
    ///
    /// When `tree` is not a build of this family.
    fn adopt(tree: &Path) -> Result<Self, Error>;

    /// Builds the family in `directory` with `hermetic`.
    ///
    /// # Errors
    ///
    /// When any part of the build fails.
    fn build_privately(
        directory: &Path,
        hermetic: &Hermetic,
        token: PrivateBuild,
    ) -> Result<Self, Error>;
}

#[derive(Debug, Clone, Copy)]
enum Acquisition {
    Adopted,
    Built,
}

impl fmt::Display for Acquisition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Adopted => "adopted",
            Self::Built => "built",
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Held {
    fixtures: Fixtures,
    acquisition: Acquisition,
}

pub struct Workbench {
    directory: TempDir,
    path: PathBuf,
    hermetic: Hermetic,
    handover: Handover,
    holds: Cell<Option<Held>>,
}

/// A fixture the workbench handed out. A private build lives inside the
/// workbench, so the fixture cannot outlive it.
pub struct Fixture<'w, F> {
    fixture: F,
    workbench: PhantomData<&'w Workbench>,
}

impl<F> Deref for Fixture<'_, F> {
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
    /// # Errors
    ///
    /// When the workbench already holds a fixture, the handed tree is missing
    /// or cannot be adopted, or the private build fails.
    pub fn shared<F: SharedFamily>(&self) -> Result<Fixture<'_, F>, Error> {
        let fixtures = Fixtures::Shared(F::FAMILY);
        self.refuse_a_second(fixtures)?;
        let test = &self.handover.test;
        let hermetic_root = self.path();
        let (fixture, acquisition) =
            if let Some(tree) = self.handover.shared_tree(F::FAMILY) {
                let fixture = adopt::<F>(tree)?;
                self.handover.log.record(&Event::Adoption {
                    family: F::FAMILY,
                    test,
                    hermetic_root,
                })?;
                (fixture, Acquisition::Adopted)
            } else {
                let fixture = F::build_privately(
                    self.path(),
                    &self.hermetic,
                    PrivateBuild(()),
                )?;
                self.handover.log.record(&Event::PrivateBuild {
                    fixtures,
                    test,
                    hermetic_root,
                })?;
                (fixture, Acquisition::Built)
            };
        self.holds.set(Some(Held {
            fixtures,
            acquisition,
        }));
        Ok(Fixture {
            fixture,
            workbench: PhantomData,
        })
    }

    fn refuse_a_second(&self, requested: Fixtures) -> Result<(), Error> {
        match self.holds.get() {
            None => Ok(()),
            Some(Held {
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
