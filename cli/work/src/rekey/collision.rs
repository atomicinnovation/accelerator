//! Targets a re-key cannot hand out because something else already names
//! them.

use crate::id_scheme::WorkItemIdScheme;
use crate::identity::holder_of_under;
use crate::identity::linker_of_under;
use crate::identity::ItemIdentity;
use crate::rekey::entry::RekeyEntry;
use crate::rekey::outcome::LinkedClash;
use crate::rekey::outcome::NotRekeyed;

/// # Errors
///
/// [`NotRekeyed::Collision`] for the first entry, by old `id`, whose old
/// `id` another item also holds, or whose target another item holds as its
/// `id`, an alias or its `external_id` under `scheme`'s taken rule, or
/// another entry also targets, ignoring case.
pub fn refuse_collisions(
    entries: &[RekeyEntry],
    items: &[ItemIdentity],
    scheme: &WorkItemIdScheme,
) -> Result<(), NotRekeyed> {
    let mut by_old: Vec<&RekeyEntry> = entries.iter().collect();
    by_old.sort_by(|left, right| left.old.cmp(&right.old));
    for (index, entry) in by_old.iter().enumerate() {
        let target = entry.target.id();
        let Some(mover) = item_holding(&entry.old, items) else {
            continue;
        };
        if let Some(namesake) = items.iter().find(|item| {
            *item != mover && item.id.eq_ignore_ascii_case(&entry.old)
        }) {
            return Err(NotRekeyed::Collision {
                first: mover.path.clone(),
                second: namesake.path.clone(),
                target: mover.id.clone(),
                linked: None,
            });
        }
        let others: Vec<ItemIdentity> = items
            .iter()
            .filter(|item| *item != mover)
            .cloned()
            .collect();
        let collision = |second: &ItemIdentity, linked| NotRekeyed::Collision {
            first: mover.path.clone(),
            second: second.path.clone(),
            target: target.to_owned(),
            linked,
        };
        if let Some(held) = holder_of_under(scheme, target, &others) {
            return Err(collision(held.item, None));
        }
        if let Some(linker) = linker_of_under(scheme, target, &others) {
            let external_id = linker.external_id.clone().unwrap_or_default();
            let shared_key = shared_key(scheme, &external_id);
            return Err(collision(
                linker,
                Some(LinkedClash {
                    external_id,
                    shared_key,
                }),
            ));
        }
        if let Some(rival) = by_old[index + 1..]
            .iter()
            .find(|later| later.target.id().eq_ignore_ascii_case(target))
            .and_then(|later| item_holding(&later.old, items))
        {
            return Err(collision(rival, None));
        }
    }
    Ok(())
}

/// `work.key`, when `external_id` reads under a key pattern with a key equal
/// to it: the tracker's own key is then `work.key`.
fn shared_key(scheme: &WorkItemIdScheme, external_id: &str) -> Option<String> {
    let key = scheme.key()?;
    let linked = scheme.read_held(external_id)?;
    linked
        .key()
        .is_some_and(|linked| linked.eq_ignore_ascii_case(key.as_str()))
        .then(|| key.as_str().to_owned())
}

fn item_holding<'a>(
    id: &str,
    items: &'a [ItemIdentity],
) -> Option<&'a ItemIdentity> {
    items.iter().find(|item| item.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::PathBuf;

    use super::refuse_collisions;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;
    use crate::id_scheme::TRACKER_TOKEN;
    use crate::identity::ItemIdentity;
    use crate::rekey::entry::RekeyEntry;
    use crate::rekey::entry::RekeyTarget;
    use crate::rekey::outcome::LinkedClash;
    use crate::rekey::outcome::NotRekeyed;
    use crate::tracker_key::TrackerKey;

    fn scheme(pattern: &str, key: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse(pattern)
                .unwrap_or_else(|error| unreachable!("{error}")),
            key.and_then(|key| IdKey::parse(key).ok()),
        )
    }

    fn acc() -> WorkItemIdScheme {
        scheme("{key}-{number:04d}", Some("ACC"))
    }

    fn pp() -> WorkItemIdScheme {
        scheme("{key}-{number:04d}", Some("PP"))
    }

    fn tracker() -> WorkItemIdScheme {
        scheme(TRACKER_TOKEN, None)
    }

    fn onto_key(old: &str, key: &str) -> RekeyEntry {
        RekeyEntry {
            old: old.to_owned(),
            target: RekeyTarget::TrackerKey(
                TrackerKey::parse(key)
                    .unwrap_or_else(|| unreachable!("a tracker key")),
            ),
        }
    }

    fn item(
        id: &str,
        aliases: &[&str],
        external_id: Option<&str>,
    ) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-item.md")),
            id: id.to_owned(),
            aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
            external_id: external_id.map(str::to_owned),
        }
    }

    fn entry(old: &str, new: &str) -> RekeyEntry {
        RekeyEntry {
            old: old.to_owned(),
            target: RekeyTarget::Renumbered(new.to_owned()),
        }
    }

    fn collision(first: &str, second: &str, target: &str) -> NotRekeyed {
        NotRekeyed::Collision {
            first: PathBuf::from(format!("meta/work/{first}-item.md")),
            second: PathBuf::from(format!("meta/work/{second}-item.md")),
            target: target.to_owned(),
            linked: None,
        }
    }

    fn linked(
        first: &str,
        second: &str,
        target: &str,
        external_id: &str,
        shared_key: Option<&str>,
    ) -> NotRekeyed {
        NotRekeyed::Collision {
            first: PathBuf::from(format!("meta/work/{first}-item.md")),
            second: PathBuf::from(format!("meta/work/{second}-item.md")),
            target: target.to_owned(),
            linked: Some(LinkedClash {
                external_id: external_id.to_owned(),
                shared_key: shared_key.map(str::to_owned),
            }),
        }
    }

    fn item_at(path: &str, id: &str) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(path),
            ..item(id, &[], None)
        }
    }

    #[test]
    fn two_items_sharing_an_id_are_refused_naming_both() {
        let items = [
            item_at("meta/work/0042-a.md", "0042"),
            item_at("meta/work/0042-b.md", "0042"),
        ];
        assert_eq!(
            refuse_collisions(
                &[entry("0042", "ACC-0042"), entry("0042", "ACC-0042")],
                &items,
                &acc()
            ),
            Err(NotRekeyed::Collision {
                first: PathBuf::from("meta/work/0042-a.md"),
                second: PathBuf::from("meta/work/0042-b.md"),
                target: "0042".to_owned(),
                linked: None,
            })
        );
    }

    #[test]
    fn two_items_sharing_an_id_ignoring_case_are_refused_naming_both() {
        let items = [
            item_at("meta/work/ACC-0042-a.md", "ACC-0042"),
            item_at("meta/work/acc-0042-b.md", "acc-0042"),
        ];
        assert_eq!(
            refuse_collisions(
                &[entry("ACC-0042", "0042"), entry("acc-0042", "0042")],
                &items,
                &acc()
            ),
            Err(NotRekeyed::Collision {
                first: PathBuf::from("meta/work/ACC-0042-a.md"),
                second: PathBuf::from("meta/work/acc-0042-b.md"),
                target: "ACC-0042".to_owned(),
                linked: None,
            })
        );
    }

    #[test]
    fn a_target_held_as_another_items_id_is_refused_naming_both() {
        let items = [item("0042", &[], None), item("ACC-0042", &[], None)];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items, &acc()),
            Err(collision("0042", "ACC-0042", "ACC-0042"))
        );
    }

    #[test]
    fn a_target_held_as_another_items_alias_is_refused_naming_both() {
        let items = [
            item("0042", &[], None),
            item("ACC-0050", &["ACC-0042"], None),
        ];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items, &acc()),
            Err(collision("0042", "ACC-0050", "ACC-0042"))
        );
    }

    #[test]
    fn two_items_sharing_a_target_are_refused_naming_both() {
        let items = [item("ACC-0042", &[], None), item("XYZ-0042", &[], None)];
        let entries = [entry("ACC-0042", "0042"), entry("XYZ-0042", "0042")];
        assert_eq!(
            refuse_collisions(&entries, &items, &acc()),
            Err(collision("ACC-0042", "XYZ-0042", "0042"))
        );
    }

    #[test]
    fn an_items_own_id_and_aliases_never_collide_with_its_target() {
        let items = [item("ACC-0042", &["0042", "acc-0042"], None)];
        assert_eq!(
            refuse_collisions(&[entry("ACC-0042", "0042")], &items, &acc()),
            Ok(())
        );
    }

    #[test]
    fn a_target_linked_by_another_items_external_id_is_refused() {
        let items = [
            item("0042", &[], None),
            item("PP-760", &[], Some("ACC-0042")),
        ];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items, &acc()),
            Err(linked(
                "0042",
                "PP-760",
                "ACC-0042",
                "ACC-0042",
                Some("ACC")
            ))
        );
    }

    #[test]
    fn a_target_held_ignoring_case_is_refused() {
        let items = [item("0042", &[], None), item("acc-0042", &[], None)];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items, &acc()),
            Err(collision("0042", "acc-0042", "ACC-0042"))
        );
    }

    #[test]
    fn a_rekey_target_held_at_another_width_collides() {
        let items = [item("0042", &[], None), item("PP-42", &[], None)];
        assert_eq!(
            refuse_collisions(&[entry("0042", "PP-0042")], &items, &pp()),
            Err(collision("0042", "PP-42", "PP-0042"))
        );
    }

    #[test]
    fn a_rekey_target_linked_at_another_width_collides() {
        let items = [
            item("0905", &[], None),
            item("PP-0001", &[], Some("PP-905")),
        ];
        assert_eq!(
            refuse_collisions(&[entry("0905", "PP-0905")], &items, &pp()),
            Err(linked("0905", "PP-0001", "PP-0905", "PP-905", Some("PP")))
        );
    }

    #[test]
    fn a_tracker_key_is_compared_whole_under_tracker() {
        let items = [
            item("0042", &[], Some("PP-760")),
            item("PP-0760", &[], None),
        ];
        assert_eq!(
            refuse_collisions(
                &[onto_key("0042", "PP-760")],
                &items,
                &tracker()
            ),
            Ok(())
        );
    }

    #[test]
    fn a_duplicate_link_keeps_todays_remedy() {
        let items = [
            item("0041", &[], Some("PP-760")),
            item("0042", &[], Some("PP-760")),
        ];
        let refused = refuse_collisions(
            &[onto_key("0041", "PP-760")],
            &items,
            &tracker(),
        );
        assert_eq!(
            refused,
            Err(linked("0041", "0042", "PP-760", "PP-760", None))
        );
        assert!(refused.err().is_some_and(|refusal| refusal
            .to_string()
            .contains("Rename or merge")));
    }

    #[test]
    fn a_rekey_onto_the_trackers_key_names_the_clash() {
        let items = [
            ItemIdentity {
                path: PathBuf::from("meta/work/0043-a.md"),
                ..item("0043", &[], None)
            },
            ItemIdentity {
                path: PathBuf::from("meta/work/0021-b\u{1b}.md"),
                ..item("0021", &[], Some("PP-43"))
            },
        ];
        let refused = refuse_collisions(
            &[entry("0021", "PP-0021"), entry("0043", "PP-0043")],
            &items,
            &pp(),
        )
        .err()
        .unwrap_or_else(|| unreachable!("a collision"));

        let message = refused.to_string();
        assert!(message.starts_with("E_REKEY_ID_COLLISION: "), "{message}");
        assert_eq!(refused.exit_code(), 4);
        for named in [
            "PP-0043",
            "meta/work/0021-bU+001B.md",
            "PP-43",
            "`work.key` `PP` is the tracker's own key",
            "choose a `work.key` the tracker does not use",
            "set `work.id_pattern` to `{tracker}`",
        ] {
            assert!(message.contains(named), "{named}: {message}");
        }
        assert!(!message.contains('\u{1b}'), "{message:?}");
    }
}
