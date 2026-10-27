//! What a re-key mapping was computed from, so an apply can tell whether its
//! preview still holds.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::PathBuf;

use corpus::WorkItemIdScheme;

use crate::identity::ItemIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityFingerprint {
    id_pattern: String,
    key: Option<String>,
    items: BTreeMap<PathBuf, ItemIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Staleness {
    SchemeChanged,
    ItemsChanged(Vec<PathBuf>),
}

impl IdentityFingerprint {
    #[must_use]
    pub fn of(scheme: &WorkItemIdScheme, items: &[ItemIdentity]) -> Self {
        Self {
            id_pattern: scheme.id_pattern.clone(),
            key: scheme.key.clone(),
            items: items
                .iter()
                .map(|item| (item.path.clone(), item.clone()))
                .collect(),
        }
    }

    #[must_use]
    pub fn scheme(&self) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: self.id_pattern.clone(),
            key: self.key.clone(),
        }
    }

    pub fn items(&self) -> impl Iterator<Item = &ItemIdentity> {
        self.items.values()
    }

    /// How the inputs differ from `earlier`'s: a changed scheme outranks any
    /// changed item, and a changed item is named by every path it was or is
    /// found at.
    #[must_use]
    pub fn staleness_since(&self, earlier: &Self) -> Option<Staleness> {
        if self.id_pattern != earlier.id_pattern || self.key != earlier.key {
            return Some(Staleness::SchemeChanged);
        }
        let changed: Vec<PathBuf> = self
            .items
            .keys()
            .chain(earlier.items.keys())
            .filter(|path| self.items.get(*path) != earlier.items.get(*path))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .cloned()
            .collect();
        (!changed.is_empty()).then_some(Staleness::ItemsChanged(changed))
    }
}

#[cfg(test)]
mod tests {
    // DSL pattern strings such as "{key}" are literal token markers, not
    // format args.
    #![allow(clippy::literal_string_with_formatting_args)]

    use std::path::PathBuf;

    use corpus::WorkItemIdScheme;

    use super::IdentityFingerprint;
    use super::Staleness;
    use crate::identity::ItemIdentity;
    use crate::work_item_files::identity_of;
    use crate::work_item_files::WorkItemFile;

    fn scheme() -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: "{key}-{number:04d}".to_owned(),
            key: Some("ACC".to_owned()),
        }
    }

    fn item(path: &str, id: &str) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(path),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: None,
        }
    }

    fn corpus() -> Vec<ItemIdentity> {
        vec![
            item("meta/work/0041-first.md", "0041"),
            item("meta/work/0042-second.md", "0042"),
        ]
    }

    fn staleness_after(
        change: impl FnOnce(&mut Vec<ItemIdentity>),
    ) -> Option<Staleness> {
        let earlier = IdentityFingerprint::of(&scheme(), &corpus());
        let mut changed = corpus();
        change(&mut changed);
        IdentityFingerprint::of(&scheme(), &changed).staleness_since(&earlier)
    }

    fn named(paths: &[&str]) -> Staleness {
        Staleness::ItemsChanged(paths.iter().map(PathBuf::from).collect())
    }

    #[test]
    fn the_fingerprint_names_each_item_whose_path_id_aliases_or_external_id_changed(
    ) {
        assert_eq!(
            staleness_after(|items| {
                items[1].path = PathBuf::from("meta/work/0042-renamed.md");
            }),
            Some(named(&[
                "meta/work/0042-renamed.md",
                "meta/work/0042-second.md"
            ]))
        );
        assert_eq!(
            staleness_after(|items| items[1].id = "0043".to_owned()),
            Some(named(&["meta/work/0042-second.md"]))
        );
        assert_eq!(
            staleness_after(|items| items[1].aliases = vec!["0007".to_owned()]),
            Some(named(&["meta/work/0042-second.md"]))
        );
        assert_eq!(
            staleness_after(|items| {
                items[0].external_id = Some("PP-760".to_owned());
            }),
            Some(named(&["meta/work/0041-first.md"]))
        );
    }

    #[test]
    fn the_fingerprint_names_an_added_item_and_a_removed_item() {
        assert_eq!(
            staleness_after(|items| {
                items.remove(0);
                items.push(item("meta/work/0043-third.md", "0043"));
            }),
            Some(named(&[
                "meta/work/0041-first.md",
                "meta/work/0043-third.md"
            ]))
        );
    }

    #[test]
    fn a_changed_scheme_is_stale_as_a_scheme_change() {
        let earlier = IdentityFingerprint::of(&scheme(), &corpus());
        let rekeyed = WorkItemIdScheme {
            key: Some("XYZ".to_owned()),
            ..scheme()
        };
        let widened = WorkItemIdScheme {
            id_pattern: "{key}-{number:05d}".to_owned(),
            ..scheme()
        };
        for changed in [rekeyed, widened] {
            assert_eq!(
                IdentityFingerprint::of(&changed, &corpus())
                    .staleness_since(&earlier),
                Some(Staleness::SchemeChanged)
            );
        }
    }

    #[test]
    fn a_body_only_edit_leaves_the_fingerprint_unchanged() {
        let file = |body: &str| WorkItemFile {
            path: PathBuf::from("meta/work/0042-second.md"),
            content: format!("---\nid: \"0042\"\n---\n\n{body}\n"),
        };
        let identities = |body: &str| {
            identity_of(&file(body)).into_iter().collect::<Vec<_>>()
        };
        let earlier = IdentityFingerprint::of(&scheme(), &identities("Before"));
        assert_eq!(
            IdentityFingerprint::of(&scheme(), &identities("After"))
                .staleness_since(&earlier),
            None
        );
    }
}
