//! Which work item a token names, read from each item's identifying fields.

use std::path::PathBuf;

use crate::id_scheme::WorkItemIdScheme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemIdentity {
    pub path: PathBuf,
    pub id: String,
    pub aliases: Vec<String>,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityField {
    Id,
    Alias,
    ExternalId,
}

impl IdentityField {
    #[must_use]
    pub const fn frontmatter_key(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Alias => "aliases",
            Self::ExternalId => "external_id",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityMatch<'a> {
    pub item: &'a ItemIdentity,
    pub field: IdentityField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityResolution<'a> {
    Unique(&'a ItemIdentity),
    Conflicting(Vec<IdentityMatch<'a>>),
    Unmatched,
}

impl ItemIdentity {
    /// The fields whose value satisfies `matches`: `id`, then `aliases`,
    /// then `external_id`.
    #[must_use]
    pub fn fields_matching(
        &self,
        matches: impl Fn(&str) -> bool,
    ) -> Vec<IdentityField> {
        let mut fields = Vec::new();
        if matches(&self.id) {
            fields.push(IdentityField::Id);
        }
        if self.aliases.iter().any(|alias| matches(alias)) {
            fields.push(IdentityField::Alias);
        }
        if self.external_id.as_deref().is_some_and(&matches) {
            fields.push(IdentityField::ExternalId);
        }
        fields
    }

    fn fields_naming(&self, token: &str) -> Vec<IdentityField> {
        self.fields_matching(|value| value.eq_ignore_ascii_case(token))
    }
}

/// Every item naming `token` through its `id`, `aliases` or `external_id`,
/// ignoring case. The token is `Unique` while one item alone names it,
/// through however many of its fields.
#[must_use]
pub fn resolve_identity<'a>(
    token: &str,
    items: &'a [ItemIdentity],
) -> IdentityResolution<'a> {
    let matches: Vec<IdentityMatch<'a>> = items
        .iter()
        .flat_map(|item| {
            item.fields_naming(token)
                .into_iter()
                .map(move |field| IdentityMatch { item, field })
        })
        .collect();
    match matches.first() {
        None => IdentityResolution::Unmatched,
        Some(first)
            if matches
                .iter()
                .all(|found| std::ptr::eq(found.item, first.item)) =>
        {
            IdentityResolution::Unique(first.item)
        }
        Some(_) => IdentityResolution::Conflicting(matches),
    }
}

/// The item already holding `candidate` as its `id` or as a retired ID in
/// its `aliases`, ignoring case.
#[must_use]
pub fn holder_of<'a>(
    candidate: &str,
    items: &'a [ItemIdentity],
) -> Option<IdentityMatch<'a>> {
    items.iter().find_map(|item| {
        item.fields_naming(candidate)
            .into_iter()
            .find(|field| *field != IdentityField::ExternalId)
            .map(|field| IdentityMatch { item, field })
    })
}

/// The item already linked to the tracker issue `key` through its
/// `external_id`, ignoring case.
#[must_use]
pub fn linker_of<'a>(
    key: &str,
    items: &'a [ItemIdentity],
) -> Option<&'a ItemIdentity> {
    items.iter().find(|item| {
        item.external_id
            .as_deref()
            .is_some_and(|linked| linked.eq_ignore_ascii_case(key))
    })
}

/// [`holder_of`] under the taken rule: a `candidate` that reads as an ID
/// is held by any value naming the same ID at any width or key case, and
/// any other candidate is compared whole, ignoring case.
#[must_use]
pub fn holder_of_under<'a>(
    scheme: &WorkItemIdScheme,
    candidate: &str,
    items: &'a [ItemIdentity],
) -> Option<IdentityMatch<'a>> {
    let Some(id) = scheme.read_held(candidate) else {
        return holder_of(candidate, items);
    };
    items.iter().find_map(|item| {
        item.fields_matching(|held| scheme.holds(held, &id))
            .into_iter()
            .find(|field| *field != IdentityField::ExternalId)
            .map(|field| IdentityMatch { item, field })
    })
}

/// [`linker_of`] under the taken rule [`holder_of_under`] applies.
#[must_use]
pub fn linker_of_under<'a>(
    scheme: &WorkItemIdScheme,
    candidate: &str,
    items: &'a [ItemIdentity],
) -> Option<&'a ItemIdentity> {
    let Some(id) = scheme.read_held(candidate) else {
        return linker_of(candidate, items);
    };
    items.iter().find(|item| {
        item.external_id
            .as_deref()
            .is_some_and(|linked| scheme.holds(linked, &id))
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::holder_of;
    use super::holder_of_under;
    use super::linker_of;
    use super::linker_of_under;
    use super::resolve_identity;
    use super::IdentityField;
    use super::IdentityResolution;
    use super::ItemIdentity;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;

    #[allow(clippy::literal_string_with_formatting_args)]
    fn keyed() -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            IdKey::parse("PP").ok(),
        )
    }

    fn item(id: &str) -> ItemIdentity {
        ItemIdentity {
            path: PathBuf::from(format!("meta/work/{id}-title.md")),
            id: id.to_owned(),
            aliases: Vec::new(),
            external_id: None,
        }
    }

    fn with_aliases(mut item: ItemIdentity, aliases: &[&str]) -> ItemIdentity {
        item.aliases = aliases.iter().map(|&alias| alias.to_owned()).collect();
        item
    }

    fn with_external_id(mut item: ItemIdentity, key: &str) -> ItemIdentity {
        item.external_id = Some(key.to_owned());
        item
    }

    fn unique_id(resolution: &IdentityResolution<'_>) -> Option<String> {
        match resolution {
            IdentityResolution::Unique(item) => Some(item.id.clone()),
            IdentityResolution::Conflicting(_)
            | IdentityResolution::Unmatched => None,
        }
    }

    #[test]
    fn a_token_resolves_by_id_alias_or_external_id_ignoring_case() {
        let items = vec![
            item("ENG-1"),
            with_aliases(item("ENG-2"), &["draft-k7mq3x"]),
            with_external_id(item("0003"), "ENG-3"),
        ];

        assert_eq!(
            unique_id(&resolve_identity("eng-1", &items)).as_deref(),
            Some("ENG-1")
        );
        assert_eq!(
            unique_id(&resolve_identity("DRAFT-K7MQ3X", &items)).as_deref(),
            Some("ENG-2")
        );
        assert_eq!(
            unique_id(&resolve_identity("eng-3", &items)).as_deref(),
            Some("0003")
        );
        assert_eq!(
            resolve_identity("ENG-4", &items),
            IdentityResolution::Unmatched
        );
    }

    #[test]
    fn a_token_matching_one_item_through_several_fields_resolves_to_it() {
        let items = vec![with_external_id(item("PP-900"), "PP-900")];

        assert_eq!(
            unique_id(&resolve_identity("pp-900", &items)).as_deref(),
            Some("PP-900")
        );
    }

    #[test]
    fn a_token_matching_different_items_through_different_fields_is_ambiguous_naming_each_field(
    ) {
        let items = vec![
            with_external_id(item("0001"), "ENG-42"),
            with_aliases(item("ENG-7"), &["ENG-42"]),
        ];

        let IdentityResolution::Conflicting(matches) =
            resolve_identity("ENG-42", &items)
        else {
            unreachable!("two items claim ENG-42")
        };

        let named: Vec<(&str, IdentityField)> = matches
            .iter()
            .map(|found| (found.item.id.as_str(), found.field))
            .collect();
        assert_eq!(
            named,
            vec![
                ("0001", IdentityField::ExternalId),
                ("ENG-7", IdentityField::Alias),
            ]
        );
    }

    #[test]
    fn each_field_is_named_by_its_frontmatter_key() {
        assert_eq!(IdentityField::Id.frontmatter_key(), "id");
        assert_eq!(IdentityField::Alias.frontmatter_key(), "aliases");
        assert_eq!(IdentityField::ExternalId.frontmatter_key(), "external_id");
    }

    #[test]
    fn resolving_pp_760_finds_legacy_item_0230_by_external_id() {
        let items = vec![
            item("0229"),
            with_external_id(item("0230"), "PP-760"),
            item("0231"),
        ];

        assert_eq!(
            unique_id(&resolve_identity("PP-760", &items)).as_deref(),
            Some("0230")
        );
    }

    #[test]
    fn a_key_already_carried_as_another_items_external_id_is_linked() {
        let items =
            vec![item("0229"), with_external_id(item("0230"), "PP-760")];

        assert_eq!(
            linker_of("pp-760", &items).map(|linker| linker.id.as_str()),
            Some("0230")
        );
        assert_eq!(linker_of("PP-761", &items), None);
    }

    #[test]
    fn a_candidate_held_as_an_id_or_alias_names_its_holder() {
        let items = vec![
            item("ENG-1"),
            with_aliases(item("ENG-2"), &["draft-k7mq3x"]),
            with_external_id(item("0003"), "ENG-3"),
        ];

        let held_as_id = holder_of("eng-1", &items)
            .map(|found| (found.item.id.as_str(), found.field));
        let held_as_alias = holder_of("DRAFT-K7MQ3X", &items)
            .map(|found| (found.item.id.as_str(), found.field));

        assert_eq!(held_as_id, Some(("ENG-1", IdentityField::Id)));
        assert_eq!(held_as_alias, Some(("ENG-2", IdentityField::Alias)));
        assert_eq!(holder_of("ENG-3", &items), None);
    }

    #[test]
    fn an_id_held_at_another_width_or_case_names_its_holder_under_a_scheme() {
        let items = vec![
            item("0007"),
            with_aliases(item("PP-0002"), &["pp-42"]),
            with_external_id(item("PP-0003"), "PP-43"),
        ];

        let holder = |candidate: &str| {
            holder_of_under(&keyed(), candidate, &items)
                .map(|found| (found.item.id.as_str(), found.field))
        };

        assert_eq!(holder("PP-0042"), Some(("PP-0002", IdentityField::Alias)));
        assert_eq!(holder("PP-0007"), Some(("0007", IdentityField::Id)));
        assert_eq!(holder("XY-0042"), None);
        assert_eq!(holder("PP-0043"), None);
        assert_eq!(
            linker_of_under(&keyed(), "PP-0043", &items)
                .map(|linker| linker.id.as_str()),
            Some("PP-0003")
        );
    }

    #[test]
    fn a_candidate_that_reads_as_no_id_is_compared_whole() {
        let items = vec![with_aliases(item("PP-0002"), &["draft-k7mq3x"])];

        assert_eq!(
            holder_of_under(&keyed(), "DRAFT-K7MQ3X", &items)
                .map(|found| found.item.id.as_str()),
            Some("PP-0002")
        );
    }
}
