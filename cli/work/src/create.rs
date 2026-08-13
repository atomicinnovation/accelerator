//! Frontmatter composition for `work create` and the template-schema
//! drift guard.
//!
//! No filesystem, no `document` dependency — `work` stays within its own
//! import-restriction rule; converting the returned [`FieldValue`] entries
//! into a `document::Yaml` mapping is the adapter's job.

use std::collections::HashSet;

/// One optional or repeated typed-linkage slot per
/// `templates/work-item.md`'s own schema.
pub struct TypedLinkage<'a> {
    pub parent: Option<&'a str>,
    pub blocks: &'a [String],
    pub blocked_by: &'a [String],
    pub derived_from: &'a [String],
    pub relates_to: &'a [String],
    pub source: Option<&'a str>,
}

/// Every already-resolved input `compose_frontmatter` needs.
pub struct CreateInputs<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub kind: &'a str,
    pub priority: &'a str,
    pub status: &'a str,
    pub linkage: TypedLinkage<'a>,
    pub tags: &'a [String],
    pub author: &'a str,
    pub producer: &'a str,
    pub date: &'a str,
    /// The remote identifier a successful `create --push` already holds by
    /// the time frontmatter is composed — `None` for a plain `create` or a
    /// `--push` that saved locally without pushing, matching the
    /// template's "omit when not linked" comment.
    pub external_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Scalar(String),
    Sequence(Vec<String>),
    Int(i64),
}

/// Every frontmatter key `compose_frontmatter` can produce.
///
/// The fixed, complete set across every optional-field combination —
/// [`assert_matches_template_schema`] compares this against the template's
/// own declared keys.
pub const KNOWN_FRONTMATTER_KEYS: &[&str] = &[
    "type",
    "id",
    "title",
    "date",
    "author",
    "producer",
    "status",
    "kind",
    "priority",
    "parent",
    "blocks",
    "blocked_by",
    "derived_from",
    "relates_to",
    "source",
    "external_id",
    "tags",
    "last_updated",
    "last_updated_by",
    "schema_version",
];

/// Builds the new item's field list, in template order.
///
/// The fixed fields, then the six typed-linkage slots (present-only for
/// `parent`/`source`, non-empty-only for
/// `blocks`/`blocked_by`/`derived_from`/`relates_to`/`tags`), then
/// `last_updated`/`last_updated_by` (identical to `date`/`author` at
/// creation time) and `schema_version: 1` unconditionally. `external_id`
/// is written only when `inputs.external_id` is `Some` — a successful
/// `create --push` — and omitted entirely otherwise, matching the
/// template's "omit when not linked" comment.
#[must_use]
pub fn compose_frontmatter(
    inputs: &CreateInputs<'_>,
) -> Vec<(String, FieldValue)> {
    let mut fields = vec![
        (
            "type".to_owned(),
            FieldValue::Scalar("work-item".to_owned()),
        ),
        ("id".to_owned(), FieldValue::Scalar(inputs.id.to_owned())),
        (
            "title".to_owned(),
            FieldValue::Scalar(inputs.title.to_owned()),
        ),
        (
            "date".to_owned(),
            FieldValue::Scalar(inputs.date.to_owned()),
        ),
        (
            "author".to_owned(),
            FieldValue::Scalar(inputs.author.to_owned()),
        ),
        (
            "producer".to_owned(),
            FieldValue::Scalar(inputs.producer.to_owned()),
        ),
        (
            "status".to_owned(),
            FieldValue::Scalar(inputs.status.to_owned()),
        ),
        (
            "kind".to_owned(),
            FieldValue::Scalar(inputs.kind.to_owned()),
        ),
        (
            "priority".to_owned(),
            FieldValue::Scalar(inputs.priority.to_owned()),
        ),
    ];

    if let Some(parent) = inputs.linkage.parent {
        fields
            .push(("parent".to_owned(), FieldValue::Scalar(parent.to_owned())));
    }
    push_sequence_if_non_empty(&mut fields, "blocks", inputs.linkage.blocks);
    push_sequence_if_non_empty(
        &mut fields,
        "blocked_by",
        inputs.linkage.blocked_by,
    );
    push_sequence_if_non_empty(
        &mut fields,
        "derived_from",
        inputs.linkage.derived_from,
    );
    push_sequence_if_non_empty(
        &mut fields,
        "relates_to",
        inputs.linkage.relates_to,
    );
    if let Some(source) = inputs.linkage.source {
        fields
            .push(("source".to_owned(), FieldValue::Scalar(source.to_owned())));
    }
    if let Some(external_id) = inputs.external_id {
        fields.push((
            "external_id".to_owned(),
            FieldValue::Scalar(external_id.to_owned()),
        ));
    }
    push_sequence_if_non_empty(&mut fields, "tags", inputs.tags);

    fields.push((
        "last_updated".to_owned(),
        FieldValue::Scalar(inputs.date.to_owned()),
    ));
    fields.push((
        "last_updated_by".to_owned(),
        FieldValue::Scalar(inputs.author.to_owned()),
    ));
    fields.push(("schema_version".to_owned(), FieldValue::Int(1)));

    fields
}

fn push_sequence_if_non_empty(
    fields: &mut Vec<(String, FieldValue)>,
    key: &str,
    values: &[String],
) {
    if !values.is_empty() {
        fields.push((key.to_owned(), FieldValue::Sequence(values.to_vec())));
    }
}

/// A source of the ambient VCS-configured user identity — injected so
/// [`resolve_author`] stays pure and testable with a double, rather than
/// `work-cli` calling a concrete adapter function directly.
pub trait VcsIdentityProbe {
    fn current_user_name(&self) -> Option<String>;
}

/// Resolves `work create`'s `author`: `explicit` (the `--author` flag) when
/// given, else `probe`'s ambient VCS identity.
///
/// # Errors
///
/// When `explicit` is absent and `probe` has no identity to offer either.
pub fn resolve_author(
    explicit: Option<&str>,
    probe: &dyn VcsIdentityProbe,
) -> Result<String, String> {
    if let Some(author) = explicit {
        return Ok(author.to_owned());
    }
    probe.current_user_name().ok_or_else(|| {
        "author could not be resolved: pass --author or run inside a \
         git/jj repository with user.name configured"
            .to_owned()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDrift {
    pub missing_from_template: Vec<String>,
    pub unknown_to_this_work_item: Vec<String>,
}

/// A structural cross-check, not a behavioural port.
///
/// Compares [`KNOWN_FRONTMATTER_KEYS`] (the fixed, complete set
/// `compose_frontmatter` knows how to produce) against
/// `templates/work-item.md`'s own declared frontmatter keys as two
/// complete sets, invocation-independent — so a future template edit is
/// caught regardless of which fields any one `work create` call supplies.
///
/// # Errors
///
/// [`SchemaDrift`] when the two sets disagree.
pub fn assert_matches_template_schema(
    template_frontmatter_keys: &[String],
) -> Result<(), SchemaDrift> {
    let known: HashSet<&str> = KNOWN_FRONTMATTER_KEYS.iter().copied().collect();
    let template: HashSet<&str> = template_frontmatter_keys
        .iter()
        .map(String::as_str)
        .collect();

    let missing_from_template: Vec<String> = KNOWN_FRONTMATTER_KEYS
        .iter()
        .filter(|key| !template.contains(*key))
        .map(|key| (*key).to_owned())
        .collect();
    let unknown_to_this_work_item: Vec<String> = template_frontmatter_keys
        .iter()
        .filter(|key| !known.contains(key.as_str()))
        .cloned()
        .collect();

    if missing_from_template.is_empty() && unknown_to_this_work_item.is_empty()
    {
        Ok(())
    } else {
        Err(SchemaDrift {
            missing_from_template,
            unknown_to_this_work_item,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assert_matches_template_schema, compose_frontmatter, resolve_author,
        CreateInputs, FieldValue, TypedLinkage, VcsIdentityProbe,
        KNOWN_FRONTMATTER_KEYS,
    };

    fn inputs<'a>(
        linkage: TypedLinkage<'a>,
        tags: &'a [String],
    ) -> CreateInputs<'a> {
        CreateInputs {
            id: "0170",
            title: "Test item",
            kind: "task",
            priority: "medium",
            status: "draft",
            linkage,
            tags,
            author: "Toby Clemson",
            producer: "accelerator-work",
            date: "2026-08-07T00:00:00+00:00",
            external_id: None,
        }
    }

    fn empty_linkage() -> TypedLinkage<'static> {
        TypedLinkage {
            parent: None,
            blocks: &[],
            blocked_by: &[],
            derived_from: &[],
            relates_to: &[],
            source: None,
        }
    }

    #[test]
    fn omits_empty_optional_fields() {
        let fields = compose_frontmatter(&inputs(empty_linkage(), &[]));
        let keys: Vec<&str> =
            fields.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "type",
                "id",
                "title",
                "date",
                "author",
                "producer",
                "status",
                "kind",
                "priority",
                "last_updated",
                "last_updated_by",
                "schema_version",
            ]
        );
    }

    #[test]
    fn includes_populated_optional_fields() {
        let blocks = vec!["work-item:0099".to_owned()];
        let tags = vec!["api".to_owned()];
        let linkage = TypedLinkage {
            parent: Some("work-item:0001"),
            blocks: &blocks,
            blocked_by: &[],
            derived_from: &[],
            relates_to: &[],
            source: Some("issue-research:0002"),
        };
        let fields = compose_frontmatter(&inputs(linkage, &tags));
        let keys: Vec<&str> =
            fields.iter().map(|(key, _)| key.as_str()).collect();
        assert!(keys.contains(&"parent"));
        assert!(keys.contains(&"blocks"));
        assert!(keys.contains(&"source"));
        assert!(keys.contains(&"tags"));
        assert!(!keys.contains(&"blocked_by"));
        assert!(!keys.contains(&"external_id"));
    }

    #[test]
    fn external_id_is_written_only_when_supplied() {
        let mut with_id = inputs(empty_linkage(), &[]);
        with_id.external_id = Some("ENG-1");
        let fields = compose_frontmatter(&with_id);
        let value = fields
            .iter()
            .find(|(key, _)| key == "external_id")
            .map(|(_, value)| value.clone());
        assert_eq!(value, Some(FieldValue::Scalar("ENG-1".to_owned())));
    }

    #[test]
    fn schema_version_is_an_unquoted_integer() {
        let fields = compose_frontmatter(&inputs(empty_linkage(), &[]));
        let schema_version = fields
            .iter()
            .find(|(key, _)| key == "schema_version")
            .map(|(_, value)| value.clone());
        assert_eq!(schema_version, Some(FieldValue::Int(1)));
    }

    #[test]
    fn schema_matches_the_known_key_set() {
        let keys: Vec<String> = KNOWN_FRONTMATTER_KEYS
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        assert!(assert_matches_template_schema(&keys).is_ok());
    }

    #[test]
    fn schema_drift_is_detected() {
        let mut keys: Vec<String> = KNOWN_FRONTMATTER_KEYS
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        keys.retain(|key| key != "priority");
        keys.push("unexpected_new_key".to_owned());
        let Err(drift) = assert_matches_template_schema(&keys) else {
            unreachable!("expected schema drift to be detected")
        };
        assert_eq!(drift.missing_from_template, vec!["priority".to_owned()]);
        assert_eq!(
            drift.unknown_to_this_work_item,
            vec!["unexpected_new_key".to_owned()]
        );
    }

    struct FixedProbe(Option<&'static str>);

    impl VcsIdentityProbe for FixedProbe {
        fn current_user_name(&self) -> Option<String> {
            self.0.map(str::to_owned)
        }
    }

    #[test]
    fn explicit_author_wins_over_the_probe() {
        let probe = FixedProbe(Some("Probe Identity"));
        assert_eq!(
            resolve_author(Some("Explicit Author"), &probe),
            Ok("Explicit Author".to_owned())
        );
    }

    #[test]
    fn falls_back_to_the_probes_identity() {
        let probe = FixedProbe(Some("Probe Identity"));
        assert_eq!(
            resolve_author(None, &probe),
            Ok("Probe Identity".to_owned())
        );
    }

    #[test]
    fn no_explicit_author_and_no_probe_identity_is_an_error() {
        let probe = FixedProbe(None);
        assert_eq!(
            resolve_author(None, &probe),
            Err("author could not be resolved: pass --author or run \
                 inside a git/jj repository with user.name configured"
                .to_owned())
        );
    }
}
