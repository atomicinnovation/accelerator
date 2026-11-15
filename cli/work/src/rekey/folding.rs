//! Which keys a re-key folds onto `work.key`, and which other keys converged
//! items keep through it.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::id_pattern::escaped;
use crate::id_pattern::IdKey;
use crate::id_scheme::IdScheme;
use crate::identity::ItemIdentity;
use crate::rekey::convergence::is_converged;
use crate::rekey::sequence::is_in_target_shape;

/// The keys whose items a re-key moves onto `work.key`.
///
/// For example, the key in use before `work.key` changed. An item
/// under any other key keeps it, since `--project` writes items under keys
/// of their own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldedKeys(Vec<IdKey>);

impl FoldedKeys {
    #[must_use]
    pub const fn of(keys: Vec<IdKey>) -> Self {
        Self(keys)
    }

    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Whether `key`, in any case, is one of these.
    #[must_use]
    pub(crate) fn folds(&self, key: &str) -> bool {
        self.0
            .iter()
            .any(|folded| folded.as_str().eq_ignore_ascii_case(key))
    }

    /// Whether a re-key leaves `identity` as it is: converged, and not under
    /// a folded key.
    #[must_use]
    pub(crate) fn keeps(
        &self,
        identity: &ItemIdentity,
        scheme: &IdScheme,
    ) -> bool {
        is_converged(identity, scheme)
            && !key_in_shape(identity, scheme)
                .is_some_and(|key| self.folds(&key))
    }
}

/// A key other than `work.key` that converged items keep through a re-key,
/// with how many hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptKey {
    pub key: String,
    pub items: usize,
}

/// The keys other than `work.key` that items keep through a re-key.
///
/// A preview names them so that a key in use before `work.key` changed can
/// be folded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptOtherKeys {
    work_key: String,
    keys: Vec<KeptKey>,
}

impl KeptOtherKeys {
    /// Every key other than `work.key` that `items` keep through a re-key
    /// folding `folded`, in upper case and key order; `None` when they keep
    /// none.
    #[must_use]
    pub fn of(
        scheme: &IdScheme,
        items: &[ItemIdentity],
        folded: &FoldedKeys,
    ) -> Option<Self> {
        let work_key = scheme.key()?.as_str().to_owned();
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for identity in items {
            if is_in_target_shape(&identity.id, scheme)
                || !folded.keeps(identity, scheme)
            {
                continue;
            }
            if let Some(key) = key_in_shape(identity, scheme) {
                *counts.entry(key.to_ascii_uppercase()).or_default() += 1;
            }
        }
        let keys: Vec<KeptKey> = counts
            .into_iter()
            .map(|(key, items)| KeptKey { key, items })
            .collect();
        (!keys.is_empty()).then_some(Self { work_key, keys })
    }

    #[must_use]
    pub fn keys(&self) -> &[KeptKey] {
        &self.keys
    }
}

impl Display for KeptOtherKeys {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let count: usize = self.keys.iter().map(|kept| kept.items).sum();
        let (noun, verb) = if count == 1 {
            ("work item", "keeps")
        } else {
            ("work items", "keep")
        };
        let keys = self
            .keys
            .iter()
            .map(|kept| format!("{} ({})", escaped(&kept.key), kept.items))
            .collect::<Vec<_>>()
            .join(", ");
        let work_key = escaped(&self.work_key);
        write!(
            formatter,
            "{count} {noun} {verb} keys other than `work.key` '{work_key}': \
             {keys}. To re-key a key's items onto '{work_key}', preview again \
             with `--from-key <KEY>`"
        )
    }
}

fn key_in_shape(identity: &ItemIdentity, scheme: &IdScheme) -> Option<String> {
    scheme
        .pattern()
        .read_in_shape(&identity.id)
        .and_then(|parts| parts.key().map(str::to_owned))
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::PathBuf;

    use super::FoldedKeys;
    use super::KeptKey;
    use super::KeptOtherKeys;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;
    use crate::identity::ItemIdentity;

    fn scheme(pattern: &str, key: Option<&str>) -> IdScheme {
        IdScheme::new(
            IdPattern::parse(pattern).unwrap_or_else(|_| unreachable!()),
            key.map(|key| IdKey::parse(key).unwrap_or_else(|_| unreachable!())),
        )
    }

    fn item(id: &str) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-item.md")),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: None,
        }
    }

    fn folded(keys: &[&str]) -> FoldedKeys {
        FoldedKeys::of(
            keys.iter()
                .map(|key| IdKey::parse(key).unwrap_or_else(|_| unreachable!()))
                .collect(),
        )
    }

    #[test]
    fn a_converged_item_is_kept_unless_its_key_is_folded() {
        let keyed = scheme("{key}-{number:04d}", Some("QQ"));

        assert!(FoldedKeys::none().keeps(&item("PP-0001"), &keyed));
        assert!(!folded(&["pp"]).keeps(&item("PP-0001"), &keyed));
        assert!(folded(&["PP"]).keeps(&item("XY-0001"), &keyed));
        assert!(!FoldedKeys::none().keeps(&item("PP-1"), &keyed));
    }

    #[test]
    fn the_other_keys_items_keep_are_counted_by_key() {
        let keyed = scheme("{key}-{number:04d}", Some("QQ"));
        let items = [
            item("QQ-0001"),
            item("PP-0002"),
            item("pp-0003"),
            item("XY-0004"),
            item("AB-5"),
        ];

        let kept = |folded: &FoldedKeys| {
            KeptOtherKeys::of(&keyed, &items, folded)
                .map(|kept| kept.keys().to_vec())
        };

        assert_eq!(
            kept(&FoldedKeys::none()),
            Some(vec![
                KeptKey {
                    key: "PP".to_owned(),
                    items: 2
                },
                KeptKey {
                    key: "XY".to_owned(),
                    items: 1
                },
            ])
        );
        assert_eq!(
            kept(&folded(&["PP"])),
            Some(vec![KeptKey {
                key: "XY".to_owned(),
                items: 1
            }])
        );
        assert_eq!(kept(&folded(&["PP", "XY"])), None);
    }

    #[test]
    fn a_pattern_without_a_key_keeps_no_other_key() {
        let bare = scheme("{number:04d}", None);

        assert_eq!(
            KeptOtherKeys::of(&bare, &[item("0001")], &FoldedKeys::none()),
            None
        );
    }
}
