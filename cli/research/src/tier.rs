//! Reputation tiers, derived from venue signals alone.

use crate::openalex::Work;
use crate::record::Tier;

/// The first matching rule wins: retraction, then peer-reviewed publication,
/// then an early version, else the lowest tier.
pub fn openalex_tier(work: &Work) -> Tier {
    if work.is_retracted {
        Tier::Three
    } else if work.is_peer_reviewed_publication() {
        Tier::One
    } else if work.is_early_version() {
        Tier::Two
    } else {
        Tier::Three
    }
}

/// arXiv's `journal_ref` and `doi` are author-supplied and unvalidated, so
/// they never raise an entry's tier.
pub const fn arxiv_tier(withdrawn: bool) -> Tier {
    if withdrawn {
        Tier::Three
    } else {
        Tier::Two
    }
}
