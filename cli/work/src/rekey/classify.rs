//! What kind of `id` a work item holds, which decides whether re-keying keeps
//! its number or hands it a fresh one.

use crate::draft_id::DraftId;
use crate::identity::ItemIdentity;
use crate::rekey::sequence::sequence_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    TrackerKey,
    Draft,
    Sequenced(u64),
    Unclassifiable,
}

/// An `id` equal to its `external_id`, ignoring case, is a tracker key; a
/// draft ID is a draft; any other `id` carrying a number keeps it.
#[must_use]
pub fn classify(identity: &ItemIdentity) -> Classification {
    let is_tracker_key = identity
        .external_id
        .as_deref()
        .is_some_and(|key| key.eq_ignore_ascii_case(&identity.id));
    if is_tracker_key {
        Classification::TrackerKey
    } else if DraftId::parse(&identity.id).is_some() {
        Classification::Draft
    } else {
        sequence_of(&identity.id)
            .map_or(Classification::Unclassifiable, Classification::Sequenced)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::classify;
    use super::Classification;
    use crate::identity::ItemIdentity;

    fn item(id: &str, external_id: Option<&str>) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-item.md")),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: external_id.map(str::to_owned),
        }
    }

    #[test]
    fn a_tracker_key_is_an_id_equal_to_its_external_id_ignoring_case() {
        assert_eq!(
            classify(&item("PP-760", Some("PP-760"))),
            Classification::TrackerKey
        );
        assert_eq!(
            classify(&item("pp-760", Some("PP-760"))),
            Classification::TrackerKey
        );
    }

    #[test]
    fn a_prefixed_id_differing_from_its_external_id_keeps_its_number() {
        assert_eq!(
            classify(&item("ACC-0042", Some("PP-760"))),
            Classification::Sequenced(42)
        );
    }

    #[test]
    fn a_tracker_shaped_id_without_an_external_id_keeps_its_number() {
        assert_eq!(
            classify(&item("PP-760", None)),
            Classification::Sequenced(760)
        );
    }

    #[test]
    fn a_draft_id_is_a_draft() {
        assert_eq!(
            classify(&item("draft-k3x9mq", None)),
            Classification::Draft
        );
    }

    #[test]
    fn an_id_of_no_known_shape_is_unclassifiable() {
        assert_eq!(
            classify(&item("notes", None)),
            Classification::Unclassifiable
        );
    }
}
