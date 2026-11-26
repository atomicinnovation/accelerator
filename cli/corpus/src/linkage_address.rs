use std::fmt;

use crate::doc_type::DocTypeKey;
use crate::linkage::linkage_id_of;
use crate::linkage_ids::LinkageIds;

/// Where a typed-linkage reference points: a doc type's linkage name and an
/// id among that type's documents, as `<type>:<id>` writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkageAddress {
    type_name: &'static str,
    id: String,
}

impl LinkageAddress {
    #[must_use]
    pub fn new(type_name: &'static str, id: impl Into<String>) -> Self {
        Self {
            type_name,
            id: id.into(),
        }
    }

    /// The address a document answers to: its frontmatter `id` when it
    /// declares one, else the id its path carries by its type's linkage id
    /// rule. `None` for a type with no linkage name.
    #[must_use]
    pub fn of_document(
        kind: DocTypeKey,
        frontmatter_id: Option<&str>,
        path: &str,
        ids: &dyn LinkageIds,
    ) -> Option<Self> {
        let type_name = kind.linkage_type_name()?;
        let id = frontmatter_id
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .or_else(|| linkage_id_of(kind, path, ids))?;
        Some(Self::new(type_name, id))
    }

    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for LinkageAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.type_name, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::LinkageAddress;
    use crate::doc_type::built_in;
    use crate::doc_type::DocTypeKey;
    use crate::linkage_ids::test_ids::KNOWN;

    fn kind(wire_token: &str) -> Option<DocTypeKey> {
        built_in::doc_types()
            .iter()
            .copied()
            .find(|kind| kind.wire_token() == wire_token)
    }

    #[test]
    fn a_document_answers_to_its_frontmatter_id() {
        let plans = kind("plans");
        assert_eq!(
            plans.and_then(|plans| LinkageAddress::of_document(
                plans,
                Some("the-plan"),
                "meta/plans/2026-01-02-renamed.md",
                &KNOWN,
            )),
            Some(LinkageAddress::new("plan", "the-plan"))
        );
    }

    #[test]
    fn a_document_without_a_frontmatter_id_answers_to_its_path_s_id() {
        let decisions = kind("decisions");
        for frontmatter_id in [None, Some("")] {
            assert_eq!(
                decisions.and_then(|decisions| LinkageAddress::of_document(
                    decisions,
                    frontmatter_id,
                    "meta/decisions/ADR-0001-example.md",
                    &KNOWN,
                )),
                Some(LinkageAddress::new("adr", "ADR-0001"))
            );
        }
    }

    #[test]
    fn a_type_without_a_linkage_name_has_no_address() {
        let templates = kind("templates");
        assert_eq!(
            templates.and_then(|templates| LinkageAddress::of_document(
                templates,
                Some("plan"),
                "templates/plan.md",
                &KNOWN,
            )),
            None
        );
    }

    #[test]
    fn an_address_writes_as_a_typed_reference() {
        assert_eq!(
            LinkageAddress::new("adr", "ADR-0001").to_string(),
            "adr:ADR-0001"
        );
    }
}
