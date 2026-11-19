//! The fixture families a run can build once and hand to every test.

use std::fmt;
use std::path::Path;

use serde::Serialize;
use serde::Serializer;

use crate::hermetic::Hermetic;
use crate::Error;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Matrix,
}

impl Family {
    pub const ALL: [Self; 1] = [Self::Matrix];

    const fn traits(self) -> Traits {
        match self {
            Self::Matrix => Traits {
                name: "matrix",
                handover_variable: "ACCELERATOR_SHARED_MATRIX",
                flag: "--matrix",
                noun: "the matrix",
            },
        }
    }

    #[must_use]
    pub const fn handover_variable(self) -> &'static str {
        self.traits().handover_variable
    }

    /// The family's name in the fixture log and on the build step's command
    /// line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.traits().name
    }

    /// The flag the build step names the family's tree with.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        self.traits().flag
    }
}

struct Traits {
    name: &'static str,
    handover_variable: &'static str,
    flag: &'static str,
    noun: &'static str,
}

impl fmt::Display for Family {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.traits().noun)
    }
}

/// What a private build made: a shared family, or the private sets that are
/// never shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltFixtures {
    Shared(Family),
    PrivateSets,
}

impl Serialize for BuiltFixtures {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::Shared(family) => family.serialize(serializer),
            Self::PrivateSets => serializer.serialize_str("private-sets"),
        }
    }
}

impl fmt::Display for BuiltFixtures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shared(family) => family.fmt(formatter),
            Self::PrivateSets => formatter.write_str("the private sets"),
        }
    }
}

/// Proof that a private build goes through a workbench, which logs it.
///
/// Only this crate can make one, so code elsewhere can implement
/// [`SharedFamily`] but never call its `build_privately`.
pub struct PrivateBuild(pub(crate) ());

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
