use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::ptr;

/// A meta document type, declared as data: its identity on each surface and
/// the facts every consumer reads from it.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct DocType {
    pub wire_token: &'static str,
    pub config_path_key: Option<&'static str>,
    /// The typed-linkage vocabulary name, as used in `<type>:<id>`
    /// references. `None` for a virtual type.
    pub linkage_type_name: Option<&'static str>,
    pub label: &'static str,
    pub in_lifecycle: bool,
    pub carries_target_frontmatter: bool,
    pub participates_in_lifecycle: bool,
    pub in_kanban: bool,
    pub is_virtual: bool,
    pub nested_root_filename: Option<&'static str>,
}

/// A handle to a declared [`DocType`], identified by its wire token.
#[derive(Clone, Copy)]
pub struct DocTypeKey(&'static DocType);

impl DocTypeKey {
    #[must_use]
    pub const fn declared(doc_type: &'static DocType) -> Self {
        Self(doc_type)
    }

    #[must_use]
    pub const fn wire_token(self) -> &'static str {
        self.0.wire_token
    }

    #[must_use]
    pub const fn config_path_key(self) -> Option<&'static str> {
        self.0.config_path_key
    }

    #[must_use]
    pub const fn linkage_type_name(self) -> Option<&'static str> {
        self.0.linkage_type_name
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        self.0.label
    }

    #[must_use]
    pub const fn in_lifecycle(self) -> bool {
        self.0.in_lifecycle
    }

    #[must_use]
    pub const fn carries_target_frontmatter(self) -> bool {
        self.0.carries_target_frontmatter
    }

    #[must_use]
    pub const fn participates_in_lifecycle(self) -> bool {
        self.0.participates_in_lifecycle
    }

    #[must_use]
    pub const fn in_kanban(self) -> bool {
        self.0.in_kanban
    }

    #[must_use]
    pub const fn is_virtual(self) -> bool {
        self.0.is_virtual
    }

    #[must_use]
    pub const fn nested_root_filename(self) -> Option<&'static str> {
        self.0.nested_root_filename
    }
}

impl PartialEq for DocTypeKey {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.0, other.0) || self.wire_token() == other.wire_token()
    }
}

impl Eq for DocTypeKey {}

impl Hash for DocTypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.wire_token().hash(state);
    }
}

impl PartialOrd for DocTypeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DocTypeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wire_token().cmp(other.wire_token())
    }
}

impl fmt::Debug for DocTypeKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.wire_token())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DocType, DocTypeKey};

    const UNREMARKABLE: DocType = DocType {
        wire_token: "",
        config_path_key: None,
        linkage_type_name: None,
        label: "",
        in_lifecycle: false,
        carries_target_frontmatter: false,
        participates_in_lifecycle: false,
        in_kanban: false,
        is_virtual: false,
        nested_root_filename: None,
    };

    static ALPHA: DocType = DocType {
        wire_token: "alpha",
        label: "Alpha",
        ..UNREMARKABLE
    };

    static ALPHA_AGAIN: DocType = DocType {
        wire_token: "alpha",
        label: "Alpha again",
        ..UNREMARKABLE
    };

    static BETA: DocType = DocType {
        wire_token: "beta",
        label: "Beta",
        ..UNREMARKABLE
    };

    #[test]
    fn keys_to_declarations_with_one_wire_token_are_equal() {
        assert_eq!(
            DocTypeKey::declared(&ALPHA),
            DocTypeKey::declared(&ALPHA_AGAIN)
        );
        assert_ne!(DocTypeKey::declared(&ALPHA), DocTypeKey::declared(&BETA));
    }

    #[test]
    fn keys_hash_and_order_by_wire_token() {
        let keys: HashSet<DocTypeKey> = [
            DocTypeKey::declared(&ALPHA),
            DocTypeKey::declared(&ALPHA_AGAIN),
            DocTypeKey::declared(&BETA),
        ]
        .into_iter()
        .collect();
        assert_eq!(keys.len(), 2);
        assert!(DocTypeKey::declared(&ALPHA) < DocTypeKey::declared(&BETA));
    }

    #[test]
    fn a_key_reads_its_declared_facts() {
        let beta = DocTypeKey::declared(&BETA);
        assert_eq!(beta.wire_token(), "beta");
        assert_eq!(beta.label(), "Beta");
        assert_eq!(format!("{beta:?}"), "beta");
    }
}
