//! Conjunctive filtering of work items by their domain attributes.
//!
//! The parent filter compares references by their identity key, so `42`,
//! `0042` and `work-item:0042` all name the same parent.

use crate::id_scheme::IdScheme;

/// The attributes a [`Filter`] matches on, borrowed from a scanned item.
#[derive(Debug, Clone, Copy)]
pub struct WorkItemView<'a> {
    pub title: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub status: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub tags: &'a [String],
    pub parent: Option<&'a str>,
}

/// The conjunctive filter parsed from the command flags. Every populated field
/// must hold for an item to be listed.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub status: Option<String>,
    pub kind: Option<String>,
    pub priority: Option<String>,
    pub parent: Option<String>,
    pub tags: Vec<String>,
    pub term: Option<String>,
}

impl Filter {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.kind.is_none()
            && self.priority.is_none()
            && self.parent.is_none()
            && self.tags.is_empty()
            && self.term.is_none()
    }

    /// True when the only populated conjunct is the free-text title term.
    #[must_use]
    pub const fn is_title_only(&self) -> bool {
        self.term.is_some()
            && self.status.is_none()
            && self.kind.is_none()
            && self.priority.is_none()
            && self.parent.is_none()
            && self.tags.is_empty()
    }

    /// True iff `item` satisfies every populated conjunct. `scheme`
    /// canonicalises both sides of the parent comparison so differing id forms
    /// that name the same work item compare equal.
    #[must_use]
    pub fn matches(&self, item: &WorkItemView<'_>, scheme: &IdScheme) -> bool {
        matches_scalar(self.status.as_deref(), item.status)
            && matches_scalar(self.kind.as_deref(), item.kind)
            && matches_scalar(self.priority.as_deref(), item.priority)
            && self.matches_parent(item.parent, scheme)
            && self.tags.iter().all(|tag| item.tags.contains(tag))
            && matches_title(self.term.as_deref(), item.title)
    }

    fn matches_parent(&self, actual: Option<&str>, scheme: &IdScheme) -> bool {
        let Some(wanted) = self.parent.as_deref() else {
            return true;
        };
        let wanted = canonical_reference(wanted, scheme);
        actual.is_some_and(|raw| canonical_reference(raw, scheme) == wanted)
    }
}

fn matches_scalar(wanted: Option<&str>, actual: Option<&str>) -> bool {
    wanted.is_none_or(|value| actual == Some(value))
}

fn matches_title(wanted: Option<&str>, actual: Option<&str>) -> bool {
    wanted.is_none_or(|term| {
        actual.is_some_and(|title| {
            title.to_lowercase().contains(&term.to_lowercase())
        })
    })
}

/// Strips a leading `type:` prefix from a typed cross-reference
/// (`work-item:0042` becomes `0042`), leaving a bare token.
#[must_use]
pub fn strip_reference_prefix(raw: &str) -> &str {
    raw.split_once(':').map_or(raw, |(_, rest)| rest).trim()
}

/// A raw work-item reference as its identity key under `scheme`, falling
/// back to the bare token when it names no ID.
#[must_use]
pub fn canonical_reference(raw: &str, scheme: &IdScheme) -> String {
    let bare = strip_reference_prefix(raw);
    scheme.identity_key(bare).unwrap_or_else(|| bare.to_owned())
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::Filter;
    use super::WorkItemView;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;

    fn scheme() -> IdScheme {
        IdScheme::numeric()
    }

    #[derive(Default)]
    struct Item {
        title: Option<String>,
        kind: Option<String>,
        status: Option<String>,
        priority: Option<String>,
        tags: Vec<String>,
        parent: Option<String>,
    }

    impl Item {
        fn view(&self) -> WorkItemView<'_> {
            WorkItemView {
                title: self.title.as_deref(),
                kind: self.kind.as_deref(),
                status: self.status.as_deref(),
                priority: self.priority.as_deref(),
                tags: &self.tags,
                parent: self.parent.as_deref(),
            }
        }
    }

    fn selected<'a>(items: &'a [Item], filter: &Filter) -> Vec<&'a Item> {
        items
            .iter()
            .filter(|item| filter.matches(&item.view(), &scheme()))
            .collect()
    }

    #[test]
    fn each_scalar_filter_selects_its_subset() {
        let bug = Item {
            kind: Some("bug".to_owned()),
            status: Some("done".to_owned()),
            ..Item::default()
        };
        let story = Item {
            kind: Some("story".to_owned()),
            ..Item::default()
        };
        let items = vec![bug, story];

        let filter = Filter {
            kind: Some("bug".to_owned()),
            ..Filter::default()
        };
        let matched = selected(&items, &filter);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].kind.as_deref(), Some("bug"));
    }

    #[test]
    fn a_multi_flag_filter_is_conjunctive() {
        let done_bug = Item {
            kind: Some("bug".to_owned()),
            status: Some("done".to_owned()),
            ..Item::default()
        };
        let draft_bug = Item {
            kind: Some("bug".to_owned()),
            status: Some("draft".to_owned()),
            ..Item::default()
        };
        let items = vec![done_bug, draft_bug];

        let filter = Filter {
            kind: Some("bug".to_owned()),
            status: Some("done".to_owned()),
            ..Filter::default()
        };
        let matched = selected(&items, &filter);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].status.as_deref(), Some("done"));
    }

    #[test]
    fn a_repeatable_tag_filter_requires_every_tag() {
        let both = Item {
            tags: vec!["backend".to_owned(), "api".to_owned()],
            ..Item::default()
        };
        let one = Item {
            tags: vec!["backend".to_owned()],
            ..Item::default()
        };
        let items = vec![both, one];

        let filter = Filter {
            tags: vec!["backend".to_owned(), "api".to_owned()],
            ..Filter::default()
        };
        let matched = selected(&items, &filter);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].tags.len(), 2);
    }

    #[test]
    fn the_parent_filter_canonicalises_both_sides() {
        let child = Item {
            title: Some("child".to_owned()),
            parent: Some("work-item:0042".to_owned()),
            ..Item::default()
        };
        let orphan = Item {
            title: Some("orphan".to_owned()),
            ..Item::default()
        };
        let items = vec![child, orphan];

        let filter = Filter {
            parent: Some("42".to_owned()),
            ..Filter::default()
        };
        let matched = selected(&items, &filter);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].title.as_deref(), Some("child"));
    }

    #[test]
    fn a_parent_filter_ignores_key_case_and_padding() {
        let keyed = IdScheme::new(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            IdKey::parse("PP").ok(),
        );
        let child = Item {
            parent: Some("work-item:PP-0042".to_owned()),
            ..Item::default()
        };
        let filter = Filter {
            parent: Some("pp-42".to_owned()),
            ..Filter::default()
        };
        assert!(filter.matches(&child.view(), &keyed));
    }

    #[test]
    fn the_title_term_is_a_case_insensitive_substring() {
        let login = Item {
            title: Some("Login Form Rework".to_owned()),
            ..Item::default()
        };
        let other = Item {
            title: Some("Sync engine".to_owned()),
            ..Item::default()
        };
        let items = vec![login, other];

        let filter = Filter {
            term: Some("login".to_owned()),
            ..Filter::default()
        };
        let matched = selected(&items, &filter);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].title.as_deref(), Some("Login Form Rework"));
    }

    #[test]
    fn a_filter_matching_nothing_selects_the_empty_set() {
        let items = vec![Item {
            kind: Some("bug".to_owned()),
            ..Item::default()
        }];
        let filter = Filter {
            kind: Some("epic".to_owned()),
            ..Filter::default()
        };
        assert!(selected(&items, &filter).is_empty());
    }
}
