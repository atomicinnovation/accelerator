//! Why re-key cannot read an item's IDs.

use std::path::Path;

use crate::draft_id::DraftId;
use crate::id_pattern::escaped;
use crate::id_pattern::is_in_id_alphabet;
use crate::id_reading::classify::classify;
use crate::id_reading::classify::Classification;
use crate::id_reading::classify::Unclassified;
use crate::id_scheme::IdOwnership;
use crate::id_scheme::IdScheme;
use crate::identity::ItemIdentity;
use crate::tracker_key::TrackerKey;

/// Why an item's `id` cannot be re-keyed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnclassifiableCause {
    AmbiguousDigitRuns { id: String, example: Option<String> },
    NoNumber { id: String },
    ForbiddenCharacter { id: String },
    ExternalIdNotATrackerKey { external_id: String },
    NumberAboveCap { id: String, number: u64, cap: u64 },
}

impl UnclassifiableCause {
    /// How the user lets re-key read the item.
    #[must_use]
    pub(crate) fn remedy(&self) -> String {
        match self {
            Self::AmbiguousDigitRuns {
                example: Some(example),
                ..
            } => format!(
                "Give the item an `id` in the shape of `work.id_pattern`, such \
                 as `{example}`"
            ),
            Self::AmbiguousDigitRuns { .. }
            | Self::NoNumber { .. } => {
                "Give the item an `id` in the shape of `work.id_pattern`".to_owned()
            }
            Self::NumberAboveCap { number, .. } => format!(
                "Widen `work.id_pattern` to hold {number}, or give the item an \
                 `id` in its shape and update the references to it"
            ),
            Self::ForbiddenCharacter { .. } => "Give the item an \
                 `id` of ASCII letters, digits, `.`, `_` and `-`"
                .to_owned(),
            Self::ExternalIdNotATrackerKey { .. } => {
                "Correct the item's `external_id` to the key of its tracker issue"
                    .to_owned()
            }
        }
    }

    /// Why re-key cannot read `item`, naming it.
    #[must_use]
    pub(crate) fn describe(&self, item: &Path) -> String {
        let item = escaped(&item.to_string_lossy());
        match self {
            Self::AmbiguousDigitRuns { id, .. } => format!(
                "{item} has `id` '{}', whose digit runs make its number ambiguous",
                escaped(id)
            ),
            Self::NoNumber { id } => {
                format!("{item} has `id` '{}', which holds no number", escaped(id))
            }
            Self::NumberAboveCap { id, number, cap } => format!(
                "{item} has `id` '{}', whose number {number} is above {cap}, the \
                 most `work.id_pattern` holds",
                escaped(id)
            ),
            Self::ForbiddenCharacter { id } => format!(
                "{item} has `id` '{}', which holds a character other than an \
                 ASCII letter, digit, `.`, `_` or `-`",
                escaped(id)
            ),
            Self::ExternalIdNotATrackerKey { external_id } => {
                format!(
                    "{item} has `external_id` '{}', which is not shaped like a \
                     tracker key",
                    escaped(external_id)
                )
            }
        }
    }
}

/// Why re-key cannot read `identity`'s IDs and so refuses it, under the
/// scheme's ownership.
#[must_use]
pub(crate) fn why_unclassifiable(
    identity: &ItemIdentity,
    scheme: &IdScheme,
) -> Option<UnclassifiableCause> {
    match scheme.ownership() {
        IdOwnership::Local => match classify(identity, scheme) {
            Classification::Unclassifiable(unclassified) => {
                Some(cause_of(unclassified, &identity.id, scheme))
            }
            Classification::Sequenced(number)
                if number > scheme.pattern().highest_number() =>
            {
                Some(UnclassifiableCause::NumberAboveCap {
                    id: identity.id.clone(),
                    number,
                    cap: scheme.pattern().highest_number(),
                })
            }
            _ => None,
        },
        IdOwnership::Tracker if DraftId::parse(&identity.id).is_some() => None,
        IdOwnership::Tracker => identity.external_id.as_deref().map_or_else(
            || {
                (!is_in_id_alphabet(&identity.id)).then(|| {
                    UnclassifiableCause::ForbiddenCharacter {
                        id: identity.id.clone(),
                    }
                })
            },
            |stored| {
                TrackerKey::parse_under(stored, scheme.tracker_keys())
                    .is_none()
                    .then(|| UnclassifiableCause::ExternalIdNotATrackerKey {
                        external_id: stored.to_owned(),
                    })
            },
        ),
    }
}

pub(crate) fn cause_of(
    unclassified: Unclassified,
    id: &str,
    scheme: &IdScheme,
) -> UnclassifiableCause {
    let id = id.to_owned();
    match unclassified {
        Unclassified::AmbiguousDigitRuns => {
            UnclassifiableCause::AmbiguousDigitRuns {
                id,
                example: scheme.pattern().render(scheme.key(), 1).ok(),
            }
        }
        Unclassified::NoNumber => UnclassifiableCause::NoNumber { id },
        Unclassified::ForbiddenCharacter => {
            UnclassifiableCause::ForbiddenCharacter { id }
        }
    }
}
