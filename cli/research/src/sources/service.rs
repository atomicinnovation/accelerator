//! The upstream services whose calls wait in a queue, each with its own
//! pacing, state directory and ticket family.

use std::num::NonZeroU8;
use std::time::Duration;

use crate::sources::pacing::Pacing;
use crate::sources::request::Family;

const ARXIV: Pacing = Pacing::previous_finish(Duration::from_secs(3));
const CROSSREF_PUBLIC: Pacing =
    Pacing::previous_start(Duration::from_millis(210), NonZeroU8::MIN);
// Ten starts must fit inside one second (9 × 102 ms = 0.918 s) while staying
// above Crossref's 100 ms polite-pool spacing.
const CROSSREF_POLITE: Pacing = Pacing::previous_start(
    Duration::from_millis(102),
    NonZeroU8::MIN.saturating_add(2),
);

/// Which of Crossref's pools serves a call: the polite pool once the caller
/// identifies itself, the public pool otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossrefPool {
    Public,
    Polite,
}

impl CrossrefPool {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Polite => "polite",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Arxiv,
    Crossref(CrossrefPool),
}

impl Service {
    pub const ALL: [Self; 3] = [
        Self::Arxiv,
        Self::Crossref(CrossrefPool::Public),
        Self::Crossref(CrossrefPool::Polite),
    ];

    pub const fn pacing(self) -> Pacing {
        match self {
            Self::Arxiv => ARXIV,
            Self::Crossref(CrossrefPool::Public) => CROSSREF_PUBLIC,
            Self::Crossref(CrossrefPool::Polite) => CROSSREF_POLITE,
        }
    }

    /// The source machine-readable output names.
    pub const fn source(self) -> &'static str {
        match self {
            Self::Arxiv => "arxiv",
            Self::Crossref(_) => "crossref",
        }
    }

    pub const fn ticket_family(self) -> Family {
        match self {
            Self::Arxiv => Family::Arxiv,
            Self::Crossref(_) => Family::Doi,
        }
    }

    /// The name of the service's state directory.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Arxiv => "arxiv",
            Self::Crossref(CrossrefPool::Public) => "crossref-public",
            Self::Crossref(CrossrefPool::Polite) => "crossref-polite",
        }
    }
}
