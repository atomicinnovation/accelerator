//! The violation taxonomy.
//!
//! Sixteen short codes carried by the frontmatter wire contract, plus two
//! later additions — `DuplicateId` (a whole-corpus collision the appending
//! index detects) and `UnquotedString` (added with the canonical-quoting
//! standard).

use std::fmt;

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
