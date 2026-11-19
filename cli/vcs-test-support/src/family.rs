//! The fixture families a run can build once and hand to every test.

use std::fmt;

use serde::Serialize;
use serde::Serializer;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Matrix,
}

impl Family {
    pub const ALL: [Self; 1] = [Self::Matrix];

    #[must_use]
    pub const fn handover_variable(self) -> &'static str {
        match self {
            Self::Matrix => "ACCELERATOR_SHARED_MATRIX",
        }
    }

    /// The family's name in the fixture log and on the build step's command
    /// line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Matrix => "matrix",
        }
    }

    /// The flag the build step names the family's tree with.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Matrix => "--matrix",
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Matrix => "the matrix",
        })
    }
}

/// What a private build made: a shared family, or the private sets that are
/// never shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixtures {
    Shared(Family),
    PrivateSets,
}

impl Serialize for Fixtures {
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

impl fmt::Display for Fixtures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shared(family) => family.fmt(formatter),
            Self::PrivateSets => formatter.write_str("the private sets"),
        }
    }
}
