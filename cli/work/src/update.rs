//! Set-key validation and list-field mutation for `work update`. No bash
//! original to port for either half — `update-work-item`'s current write
//! step does this via ad-hoc `Edit` calls, not a script.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    IdImmutable,
    ScalarSetOnListField { key: String },
}

/// The typed-linkage fields that are YAML sequences, not scalars —
/// `--set` on any of these (or on `tags`) is rejected before any mutation
/// is applied.
pub const LIST_FIELDS: &[&str] =
    &["blocks", "blocked_by", "derived_from", "relates_to"];

/// Rejects the own-identity fields and every list-typed field (`tags` and
/// [`LIST_FIELDS`]) as a `--set` target.
///
/// # Errors
///
/// [`UpdateError::IdImmutable`] for `id`/`work_item_id`;
/// [`UpdateError::ScalarSetOnListField`] for `tags` or a [`LIST_FIELDS`]
/// member.
pub fn validate_set_key(key: &str) -> Result<(), UpdateError> {
    if key == "id" || key == "work_item_id" {
        return Err(UpdateError::IdImmutable);
    }
    if key == "tags" || LIST_FIELDS.contains(&key) {
        return Err(UpdateError::ScalarSetOnListField {
            key: key.to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListAction {
    Append,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMutation {
    Changed(Vec<String>),
    NoChange,
}

/// Adds or removes `value` from `current` — add-if-absent,
/// remove-if-present, no bash quirk to preserve (these fields were only
/// ever edited via the skill's generic `Edit` flow).
#[must_use]
pub fn mutate_list(
    current: &[String],
    action: ListAction,
    value: &str,
) -> ListMutation {
    match action {
        ListAction::Append => {
            if current.iter().any(|existing| existing == value) {
                return ListMutation::NoChange;
            }
            let mut next = current.to_vec();
            next.push(value.to_owned());
            ListMutation::Changed(next)
        }
        ListAction::Remove => {
            if !current.iter().any(|existing| existing == value) {
                return ListMutation::NoChange;
            }
            ListMutation::Changed(
                current
                    .iter()
                    .filter(|existing| *existing != value)
                    .cloned()
                    .collect(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mutate_list, validate_set_key, ListAction, ListMutation, UpdateError,
        LIST_FIELDS,
    };

    #[test]
    fn own_identity_fields_are_hard_blocked() {
        assert_eq!(validate_set_key("id"), Err(UpdateError::IdImmutable));
        assert_eq!(
            validate_set_key("work_item_id"),
            Err(UpdateError::IdImmutable)
        );
    }

    #[test]
    fn tags_and_list_fields_are_rejected() {
        assert_eq!(
            validate_set_key("tags"),
            Err(UpdateError::ScalarSetOnListField {
                key: "tags".to_owned()
            })
        );
        for field in LIST_FIELDS {
            assert_eq!(
                validate_set_key(field),
                Err(UpdateError::ScalarSetOnListField {
                    key: (*field).to_owned()
                })
            );
        }
    }

    #[test]
    fn ordinary_scalar_fields_are_accepted() {
        assert_eq!(validate_set_key("status"), Ok(()));
        assert_eq!(validate_set_key("priority"), Ok(()));
        assert_eq!(validate_set_key("external_id"), Ok(()));
    }

    #[test]
    fn mutate_list_append_new_and_duplicate() {
        let current = vec!["work-item:0001".to_owned()];
        assert_eq!(
            mutate_list(&current, ListAction::Append, "work-item:0002"),
            ListMutation::Changed(vec![
                "work-item:0001".to_owned(),
                "work-item:0002".to_owned()
            ])
        );
        assert_eq!(
            mutate_list(&current, ListAction::Append, "work-item:0001"),
            ListMutation::NoChange
        );
    }

    #[test]
    fn mutate_list_remove_present_and_absent() {
        let current =
            vec!["work-item:0001".to_owned(), "work-item:0002".to_owned()];
        assert_eq!(
            mutate_list(&current, ListAction::Remove, "work-item:0001"),
            ListMutation::Changed(vec!["work-item:0002".to_owned()])
        );
        assert_eq!(
            mutate_list(&current, ListAction::Remove, "work-item:0099"),
            ListMutation::NoChange
        );
    }

    #[test]
    fn mutate_list_append_on_a_missing_key_treats_it_as_empty() {
        assert_eq!(
            mutate_list(&[], ListAction::Append, "work-item:0001"),
            ListMutation::Changed(vec!["work-item:0001".to_owned()])
        );
    }
}
