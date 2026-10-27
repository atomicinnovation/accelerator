//! Targets a re-key cannot hand out because something else already names
//! them.

use crate::identity::holder_of;
use crate::identity::linker_of;
use crate::identity::ItemIdentity;
use crate::rekey::entry::RekeyEntry;
use crate::rekey::outcome::NotRekeyed;

/// # Errors
///
/// [`NotRekeyed::Collision`] for the first entry, by old `id`, whose old
/// `id` another item also holds, or whose target another item holds as its
/// `id`, an alias or its `external_id`, or another entry also targets,
/// ignoring case.
pub fn refuse_collisions(
    entries: &[RekeyEntry],
    items: &[ItemIdentity],
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
            });
        }
        let others: Vec<ItemIdentity> = items
            .iter()
            .filter(|item| *item != mover)
            .cloned()
            .collect();
        let claimant = holder_of(target, &others)
            .map(|held| held.item.path.clone())
            .or_else(|| {
                linker_of(target, &others).map(|item| item.path.clone())
            })
            .or_else(|| {
                by_old[index + 1..]
                    .iter()
                    .find(|later| {
                        later.target.id().eq_ignore_ascii_case(target)
                    })
                    .and_then(|later| item_holding(&later.old, items))
                    .map(|item| item.path.clone())
            });
        if let Some(second) = claimant {
            return Err(NotRekeyed::Collision {
                first: mover.path.clone(),
                second,
                target: target.to_owned(),
            });
        }
    }
    Ok(())
}

fn item_holding<'a>(
    id: &str,
    items: &'a [ItemIdentity],
) -> Option<&'a ItemIdentity> {
    items.iter().find(|item| item.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::refuse_collisions;
    use crate::identity::ItemIdentity;
    use crate::rekey::entry::RekeyEntry;
    use crate::rekey::entry::RekeyTarget;
    use crate::rekey::outcome::NotRekeyed;

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
                &items
            ),
            Err(NotRekeyed::Collision {
                first: PathBuf::from("meta/work/0042-a.md"),
                second: PathBuf::from("meta/work/0042-b.md"),
                target: "0042".to_owned(),
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
                &items
            ),
            Err(NotRekeyed::Collision {
                first: PathBuf::from("meta/work/ACC-0042-a.md"),
                second: PathBuf::from("meta/work/acc-0042-b.md"),
                target: "ACC-0042".to_owned(),
            })
        );
    }

    #[test]
    fn a_target_held_as_another_items_id_is_refused_naming_both() {
        let items = [item("0042", &[], None), item("ACC-0042", &[], None)];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items),
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
            refuse_collisions(&[entry("0042", "ACC-0042")], &items),
            Err(collision("0042", "ACC-0050", "ACC-0042"))
        );
    }

    #[test]
    fn two_items_sharing_a_target_are_refused_naming_both() {
        let items = [item("ACC-0042", &[], None), item("XYZ-0042", &[], None)];
        let entries = [entry("ACC-0042", "0042"), entry("XYZ-0042", "0042")];
        assert_eq!(
            refuse_collisions(&entries, &items),
            Err(collision("ACC-0042", "XYZ-0042", "0042"))
        );
    }

    #[test]
    fn an_items_own_id_and_aliases_never_collide_with_its_target() {
        let items = [item("ACC-0042", &["0042", "acc-0042"], None)];
        assert_eq!(
            refuse_collisions(&[entry("ACC-0042", "0042")], &items),
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
            refuse_collisions(&[entry("0042", "ACC-0042")], &items),
            Err(collision("0042", "PP-760", "ACC-0042"))
        );
    }

    #[test]
    fn a_target_held_ignoring_case_is_refused() {
        let items = [item("0042", &[], None), item("acc-0042", &[], None)];
        assert_eq!(
            refuse_collisions(&[entry("0042", "ACC-0042")], &items),
            Err(collision("0042", "acc-0042", "ACC-0042"))
        );
    }
}
