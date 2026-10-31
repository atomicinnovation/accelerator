//! What kind of `id` a work item holds, which decides whether re-keying keeps
//! its number or hands it a fresh one.

use crate::draft_id::DraftId;
use crate::id_scheme::WorkItemIdScheme;
use crate::identity::ItemIdentity;
use crate::rekey::sequence::read_existing_id;
pub use crate::rekey::sequence::Unclassified;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    TrackerKey,
    Draft,
    Sequenced(u64),
    Unclassifiable(Unclassified),
}

/// An `id` equal to its `external_id`, ignoring case, is a tracker key; a
/// draft ID is a draft; any other `id` carrying a number keeps it.
#[must_use]
pub fn classify(
    identity: &ItemIdentity,
    scheme: &WorkItemIdScheme,
) -> Classification {
    let is_tracker_key = identity
        .external_id
        .as_deref()
        .is_some_and(|key| key.eq_ignore_ascii_case(&identity.id));
    if is_tracker_key {
        Classification::TrackerKey
    } else if DraftId::parse(&identity.id).is_some() {
        Classification::Draft
    } else {
        read_existing_id(&identity.id, scheme)
            .map_or_else(Classification::Unclassifiable, |existing| {
                Classification::Sequenced(existing.number)
            })
    }
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::PathBuf;

    use super::classify;
    use super::Classification;
    use super::Unclassified;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;
    use crate::identity::ItemIdentity;

    fn item(id: &str, external_id: Option<&str>) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-item.md")),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: external_id.map(str::to_owned),
        }
    }

    fn keyed() -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            IdKey::parse("PP").ok(),
        )
    }

    fn classified(id: &str, external_id: Option<&str>) -> Classification {
        classify(&item(id, external_id), &keyed())
    }

    #[test]
    fn a_tracker_key_is_an_id_equal_to_its_external_id_ignoring_case() {
        assert_eq!(
            classified("PP-760", Some("PP-760")),
            Classification::TrackerKey
        );
        assert_eq!(
            classified("pp-760", Some("PP-760")),
            Classification::TrackerKey
        );
    }

    #[test]
    fn a_prefixed_id_differing_from_its_external_id_keeps_its_number() {
        assert_eq!(
            classified("ACC-0042", Some("PP-760")),
            Classification::Sequenced(42)
        );
    }

    #[test]
    fn a_tracker_shaped_id_without_an_external_id_keeps_its_number() {
        assert_eq!(classified("PP-760", None), Classification::Sequenced(760));
    }

    #[test]
    fn a_draft_id_is_a_draft() {
        assert_eq!(classified("draft-k3x9mq", None), Classification::Draft);
    }

    #[test]
    fn an_id_of_no_known_shape_is_unclassifiable() {
        assert_eq!(
            classified("notes", None),
            Classification::Unclassifiable(Unclassified::NoNumber)
        );
    }

    #[test]
    fn an_id_written_under_another_pattern_keeps_its_number() {
        for id in ["v001", "PP_0001"] {
            assert_eq!(
                classified(id, None),
                Classification::Sequenced(1),
                "{id}"
            );
        }
        for id in ["PP2_0001", "0001v2"] {
            assert_eq!(
                classified(id, None),
                Classification::Unclassifiable(
                    Unclassified::AmbiguousDigitRuns
                ),
                "{id}"
            );
        }
        for id in ["a/b/0001", "x\u{1b}[H0001"] {
            assert_eq!(
                classified(id, None),
                Classification::Unclassifiable(
                    Unclassified::NotOnePathComponent
                ),
                "{id:?}"
            );
        }
    }
}
