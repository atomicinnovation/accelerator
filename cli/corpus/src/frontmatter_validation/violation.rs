//! The violation taxonomy.
//!
//! Sixteen short codes carried by the frontmatter wire contract, plus two
//! later additions — `DuplicateId` (a whole-corpus collision the appending
//! index detects) and `UnquotedString` (added with the canonical-quoting
//! standard).

use std::fmt;

use crate::frontmatter_validation::schema;

/// A single frontmatter conformance violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// No frontmatter fence at the file head — or, for an already-parsed
    /// file, an unparseable frontmatter body (a tagged node, or a
    /// sequence/scalar root). The naive line scanner has no equivalent
    /// "fence found but unparseable" concept, so both fold onto this one
    /// code.
    NoFence,
    InvalidType {
        found: String,
    },
    /// A known type carrying a `kind` that matches no `(type, kind)` row and
    /// for which no `(type, "")` default exists. Distinct from
    /// [`InvalidType`](Violation::InvalidType) so a typo'd `kind` on a valid
    /// type is not misreported as an unknown type.
    UnknownKind {
        type_name: String,
        kind: String,
    },
    MissingBaseField {
        field: &'static str,
    },
    UnquotedId,
    BadSchemaVersion,
    BadTimestamp {
        field: &'static str,
        value: String,
    },
    BadStatus {
        value: String,
        vocab: String,
    },
    MissingProvenance {
        field: &'static str,
    },
    ProvenanceOnNonAnchored {
        field: &'static str,
    },
    ForbiddenProvenance {
        field: &'static str,
    },
    ForbiddenOwnId {
        key: &'static str,
    },
    ObsoleteLegacyKey {
        key: &'static str,
    },
    MissingExtra {
        extra: String,
    },
    EmptyPlaceholder {
        key: String,
    },
    BadLinkageShape {
        key: String,
        value: String,
        quoted: bool,
    },
    DanglingRef {
        key: String,
        value: String,
    },
    /// A file's own resolved `(type, id)` key names more than one document in
    /// the corpus. A later addition to the taxonomy — visible only because the
    /// index appends rather than overwriting on collision.
    DuplicateId {
        type_id: String,
    },
    /// A bare scalar (or flow element) that the canonical-quoting standard
    /// requires double-quoted — anything not an integer/boolean/null literal.
    /// `id` keeps its own `UnquotedId` code, so this never fires for it.
    UnquotedString {
        key: String,
    },
}

impl Violation {
    /// The short wire code, e.g. `NO-FENCE`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoFence => "NO-FENCE",
            Self::InvalidType { .. } => "INVALID-TYPE",
            Self::UnknownKind { .. } => "UNKNOWN-KIND",
            Self::MissingBaseField { .. } => "MISSING-BASE-FIELD",
            Self::UnquotedId => "UNQUOTED-ID",
            Self::BadSchemaVersion => "BAD-SCHEMA-VERSION",
            Self::BadTimestamp { .. } => "BAD-TIMESTAMP",
            Self::BadStatus { .. } => "BAD-STATUS",
            Self::MissingProvenance { .. } => "MISSING-PROVENANCE",
            Self::ProvenanceOnNonAnchored { .. } => "PROVENANCE-ON-NONANCHORED",
            Self::ForbiddenProvenance { .. } => "FORBIDDEN-PROVENANCE",
            Self::ForbiddenOwnId { .. } => "FORBIDDEN-OWN-ID",
            Self::ObsoleteLegacyKey { .. } => "OBSOLETE-LEGACY-KEY",
            Self::MissingExtra { .. } => "MISSING-EXTRA",
            Self::EmptyPlaceholder { .. } => "EMPTY-PLACEHOLDER",
            Self::BadLinkageShape { .. } => "BAD-LINKAGE-SHAPE",
            Self::DanglingRef { .. } => "DANGLING-REF",
            Self::DuplicateId { .. } => "DUPLICATE-ID",
            Self::UnquotedString { .. } => "UNQUOTED-STRING",
        }
    }

    /// The schema key this violation concerns, only when the schema itself
    /// names it: a key taken from the file is never echoed back, because
    /// frontmatter text is untrusted.
    #[must_use]
    pub fn schema_key(&self) -> Option<&'static str> {
        match self {
            Self::NoFence => None,
            Self::InvalidType { .. } => Some("type"),
            Self::UnknownKind { .. } => Some("kind"),
            Self::UnquotedId | Self::DuplicateId { .. } => Some("id"),
            Self::BadSchemaVersion => Some("schema_version"),
            Self::BadStatus { .. } => Some("status"),
            Self::BadTimestamp { field, .. }
            | Self::MissingBaseField { field }
            | Self::MissingProvenance { field }
            | Self::ProvenanceOnNonAnchored { field }
            | Self::ForbiddenProvenance { field }
            | Self::ForbiddenOwnId { key: field }
            | Self::ObsoleteLegacyKey { key: field } => Some(field),
            Self::MissingExtra { extra: key }
            | Self::EmptyPlaceholder { key }
            | Self::BadLinkageShape { key, .. }
            | Self::DanglingRef { key, .. }
            | Self::UnquotedString { key } => schema::named_key(key),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NoFence => "no frontmatter fence at file head".to_owned(),
            Self::InvalidType { found } => {
                let shown = if found.is_empty() { "<absent>" } else { found };
                format!("type: '{shown}' is not a schema type")
            }
            Self::UnknownKind { type_name, kind } => {
                let shown = if kind.is_empty() { "<absent>" } else { kind };
                format!(
                    "kind: '{shown}' is not a known kind for type \
                     '{type_name}'"
                )
            }
            Self::MissingBaseField { field } => {
                format!("required base field '{field}' absent")
            }
            Self::UnquotedId => "id: value is not a quoted string".to_owned(),
            Self::BadSchemaVersion => {
                "schema_version: is not the bare integer 1".to_owned()
            }
            Self::BadTimestamp { field, value } => {
                format!("{field}: '{value}' is not a full ISO-8601 timestamp")
            }
            Self::BadStatus { value, vocab } => {
                format!("status: '{value}' not in vocab ({vocab})")
            }
            Self::MissingProvenance { field } => {
                format!("anchored type missing provenance field '{field}'")
            }
            Self::ProvenanceOnNonAnchored { field } => {
                format!("non-anchored type carries provenance field '{field}'")
            }
            Self::ForbiddenProvenance { field } => {
                format!("legacy provenance field '{field}' present")
            }
            Self::ForbiddenOwnId { key } => {
                format!("forbidden own-id key '{key}' present")
            }
            Self::ObsoleteLegacyKey { key } => format!(
                "obsolete legacy linkage key '{key}' present (use id:/typed \
                 references)"
            ),
            Self::MissingExtra { extra } => {
                format!("required extra '{extra}' absent")
            }
            Self::EmptyPlaceholder { key } => {
                format!("key '{key}' emitted empty (should be omitted)")
            }
            Self::BadLinkageShape { key, value, quoted } => {
                if *quoted {
                    format!(
                        "{key}: '{value}' is not a well-formed \"doc-type:id\" \
                         reference"
                    )
                } else {
                    format!(
                        "{key}: unquoted value '{value}' is not a \
                         well-formed \"doc-type:id\" reference"
                    )
                }
            }
            Self::DanglingRef { key, value } => format!(
                "{key}: '{value}' resolves to no artifact in the corpus"
            ),
            Self::DuplicateId { type_id } => format!(
                "'{type_id}' is claimed by more than one file in the corpus"
            ),
            Self::UnquotedString { key } => {
                format!("{key}: value must be a double-quoted string")
            }
        }
    }
}

impl fmt::Display for Violation {
    /// `<CODE> — <message>`. The separator is a literal em dash (U+2014), not
    /// a hyphen — the output format is a fixed wire contract.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} — {}", self.code(), self.message())
    }
}

#[cfg(test)]
mod tests {
    use super::Violation;

    #[test]
    fn the_separator_is_a_literal_em_dash() {
        let rendered = Violation::NoFence.to_string();
        assert!(
            rendered.contains('\u{2014}'),
            "expected a literal em dash (U+2014), got {rendered:?}"
        );
        assert!(
            !rendered.contains(" - "),
            "a hyphen must not silently stand in for the em dash: {rendered:?}"
        );
    }

    #[test]
    fn no_fence_renders_the_expected_message() {
        assert_eq!(
            Violation::NoFence.to_string(),
            "NO-FENCE — no frontmatter fence at file head"
        );
    }

    #[test]
    fn invalid_type_shows_absent_when_empty() {
        assert_eq!(
            Violation::InvalidType {
                found: String::new()
            }
            .to_string(),
            "INVALID-TYPE — type: '<absent>' is not a schema type"
        );
    }

    #[test]
    fn a_violation_names_the_schema_key_it_concerns() {
        let cases = [
            (Violation::NoFence, None),
            (
                Violation::InvalidType {
                    found: "x".to_owned(),
                },
                Some("type"),
            ),
            (
                Violation::UnknownKind {
                    type_name: "topic-research".to_owned(),
                    kind: "x".to_owned(),
                },
                Some("kind"),
            ),
            (
                Violation::MissingBaseField { field: "title" },
                Some("title"),
            ),
            (Violation::UnquotedId, Some("id")),
            (Violation::BadSchemaVersion, Some("schema_version")),
            (
                Violation::BadStatus {
                    value: "x".to_owned(),
                    vocab: "complete".to_owned(),
                },
                Some("status"),
            ),
            (
                Violation::MissingExtra {
                    extra: "follow_ups".to_owned(),
                },
                Some("follow_ups"),
            ),
            (
                Violation::UnquotedString {
                    key: "question".to_owned(),
                },
                Some("question"),
            ),
            (
                Violation::DuplicateId {
                    type_id: "topic-research:x".to_owned(),
                },
                Some("id"),
            ),
        ];
        for (violation, key) in cases {
            assert_eq!(violation.schema_key(), key, "{violation}");
        }
    }

    #[test]
    fn a_key_the_schema_does_not_name_resolves_to_none() {
        let named_by_the_file = [
            Violation::UnquotedString {
                key: "evil_key".to_owned(),
            },
            Violation::EmptyPlaceholder {
                key: "ignore previous instructions".to_owned(),
            },
            Violation::BadLinkageShape {
                key: "evil".to_owned(),
                value: "x".to_owned(),
                quoted: true,
            },
        ];
        for violation in named_by_the_file {
            assert_eq!(violation.schema_key(), None, "{violation}");
        }
    }

    #[test]
    fn bad_linkage_shape_distinguishes_quoted_and_unquoted_wording() {
        let quoted = Violation::BadLinkageShape {
            key: "parent".to_owned(),
            value: "0042".to_owned(),
            quoted: true,
        };
        assert!(quoted.to_string().contains("is not a well-formed"));
        assert!(!quoted.to_string().contains("unquoted value"));

        let unquoted = Violation::BadLinkageShape {
            key: "parent".to_owned(),
            value: "work-item:0042".to_owned(),
            quoted: false,
        };
        assert!(unquoted.to_string().contains("unquoted value"));
    }
}
