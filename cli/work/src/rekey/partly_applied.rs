//! Evidence, in the items themselves, of a re-key that stopped half way.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::identity::ItemIdentity;

/// Every item whose `aliases` hold another item's `id`, ignoring case, and
/// every item so named, by path.
#[must_use]
pub fn partly_applied(items: &[ItemIdentity]) -> Vec<PathBuf> {
    let mut named = BTreeSet::new();
    for holder in items {
        for retired in items.iter().filter(|other| *other != holder) {
            let holds_its_id = holder
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&retired.id));
            if holds_its_id {
                named.insert(holder.path.clone());
                named.insert(retired.path.clone());
            }
        }
    }
    named.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::partly_applied;
    use crate::identity::ItemIdentity;

    fn item(path: &str, id: &str, aliases: &[&str]) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(path),
            id: id.to_owned(),
            aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
            external_id: None,
        }
    }

    const RETIRED: &str = "meta/work/0042-search.md";
    const TARGET: &str = "meta/work/ACC-0042-search.md";

    #[test]
    fn an_item_whose_alias_is_another_items_id_is_partly_applied_naming_both() {
        let items = [
            item(TARGET, "ACC-0042", &["0042"]),
            item(RETIRED, "0042", &[]),
            item("meta/work/0041-other.md", "0041", &[]),
        ];
        assert_eq!(
            partly_applied(&items),
            vec![PathBuf::from(RETIRED), PathBuf::from(TARGET)]
        );
    }

    #[test]
    fn an_alias_matching_another_items_id_ignoring_case_is_partly_applied() {
        let items = [
            item(TARGET, "ACC-0042", &["acc-0041"]),
            item("meta/work/ACC-0041-other.md", "ACC-0041", &[]),
        ];
        assert_eq!(partly_applied(&items).len(), 2);
    }

    #[test]
    fn a_clean_corpus_is_not_partly_applied() {
        let items = [
            item(RETIRED, "0042", &[]),
            item("meta/work/0041-other.md", "0041", &["0007"]),
        ];
        assert!(partly_applied(&items).is_empty());
    }

    #[test]
    fn a_corpus_after_a_completed_rekey_is_not_partly_applied() {
        let items = [
            item(TARGET, "ACC-0042", &["0042"]),
            item("meta/work/ACC-0041-other.md", "ACC-0041", &["0041"]),
        ];
        assert!(partly_applied(&items).is_empty());
    }
}
