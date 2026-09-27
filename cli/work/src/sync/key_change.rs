//! What a work item does when its remote issue now answers to another key.

use corpus::IdOwnership;
use tracker::ExternalId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyChange {
    /// Only `external_id` follows the new key; the item keeps its `id`.
    FollowExternalId {
        item: String,
        old: ExternalId,
        new: ExternalId,
    },
    /// The `id` is the old key, so it is retired in favour of the new one.
    RetireKey {
        item: String,
        old: ExternalId,
        new: ExternalId,
    },
}

impl KeyChange {
    #[must_use]
    pub fn item(&self) -> &str {
        match self {
            Self::FollowExternalId { item, .. }
            | Self::RetireKey { item, .. } => item,
        }
    }

    #[must_use]
    pub const fn old(&self) -> &ExternalId {
        match self {
            Self::FollowExternalId { old, .. }
            | Self::RetireKey { old, .. } => old,
        }
    }

    #[must_use]
    pub const fn new_key(&self) -> &ExternalId {
        match self {
            Self::FollowExternalId { new, .. }
            | Self::RetireKey { new, .. } => new,
        }
    }
}

#[must_use]
pub fn decide_key_change(
    ownership: IdOwnership,
    item_id: &str,
    old: &ExternalId,
    new: &ExternalId,
) -> KeyChange {
    let item = item_id.to_owned();
    let (old, new) = (old.clone(), new.clone());
    let id_is_the_old_key = item_id.eq_ignore_ascii_case(old.as_str());
    if ownership == IdOwnership::Tracker && id_is_the_old_key {
        KeyChange::RetireKey { item, old, new }
    } else {
        KeyChange::FollowExternalId { item, old, new }
    }
}

#[cfg(test)]
mod tests {
    use corpus::IdOwnership;
    use tracker::ExternalId;

    use super::decide_key_change;
    use super::KeyChange;

    fn key(raw: &str) -> ExternalId {
        ExternalId::new(raw.to_owned())
    }

    #[test]
    fn local_ownership_always_follows_external_id() {
        for item in ["0230", "PP-760"] {
            let decided = decide_key_change(
                IdOwnership::Local,
                item,
                &key("PP-760"),
                &key("ENG-42"),
            );

            assert!(
                matches!(decided, KeyChange::FollowExternalId { .. }),
                "{item}: {decided:?}"
            );
        }
    }

    #[test]
    fn tracker_ownership_retires_when_id_equals_the_old_key_in_any_case() {
        for item in ["PP-760", "pp-760"] {
            let decided = decide_key_change(
                IdOwnership::Tracker,
                item,
                &key("PP-760"),
                &key("ENG-42"),
            );

            assert_eq!(
                decided,
                KeyChange::RetireKey {
                    item: item.to_owned(),
                    old: key("PP-760"),
                    new: key("ENG-42"),
                }
            );
        }
    }

    #[test]
    fn tracker_ownership_follows_external_id_when_id_differs_from_the_old_key()
    {
        let decided = decide_key_change(
            IdOwnership::Tracker,
            "0230",
            &key("PP-760"),
            &key("ENG-42"),
        );

        assert!(
            matches!(decided, KeyChange::FollowExternalId { .. }),
            "{decided:?}"
        );
    }
}
