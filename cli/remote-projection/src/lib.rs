//! The per-tracker remote-payload projection seam.
//!
//! A crate of its own rather than a module of `work-adapters`: the provider
//! clients and the sync engine both project, and this is JSON field
//! extraction rather than a domain decision — typing it against
//! `serde_json::Value` would need a dependency `work`'s own
//! import-restriction rule does not permit.
//!
//! [`project`] deliberately emits **no** trailing newline. The committed
//! parity fixtures reconstruct the expected body line-wise and carry none
//! either, so the asymmetry is load-bearing here; a caller populating
//! `tracker::RemoteIssue.body`, whose port contract requires the newline,
//! appends it.

pub mod json;

use serde_json::Value;

use crate::json::JsonError;
use crate::json::Limits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integration {
    Jira,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Updated,
    Body,
}

/// Parses `--integration <string>`, accepting only `jira` and `linear`.
#[must_use]
pub fn parse_integration(value: &str) -> Option<Integration> {
    match value {
        "jira" => Some(Integration::Jira),
        "linear" => Some(Integration::Linear),
        _ => None,
    }
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

/// Compact, key-sorted canonicalisation. `serde_json` without the
/// `preserve_order` feature already backs its object type with a `BTreeMap`,
/// so `to_string` alone gives key-sorted, whitespace-free output — no extra
/// sorting step needed.
fn canonicalise(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Projects `remote_json` into the comparable local shape. Always
/// infallible: a missing field yields an empty result rather than an error.
#[must_use]
pub fn project(
    integration: Integration,
    op: Op,
    remote_json: &Value,
) -> String {
    match (integration, op) {
        (Integration::Jira, Op::Updated) => {
            string_at(remote_json, "/fields/updated").to_owned()
        }
        (Integration::Jira, Op::Body) => {
            let summary = string_at(remote_json, "/fields/summary");
            let description = remote_json
                .pointer("/fields/description")
                .cloned()
                .unwrap_or(Value::Null);
            format!("{summary}\n{}", canonicalise(&description))
        }
        (Integration::Linear, Op::Updated) => {
            string_at(remote_json, "/data/issue/updatedAt").to_owned()
        }
        (Integration::Linear, Op::Body) => {
            let title = string_at(remote_json, "/data/issue/title");
            let description = string_at(remote_json, "/data/issue/description");
            format!("{title}\n{description}")
        }
    }
}

/// The same projection, from the response's raw bytes rather than a parsed
/// `Value`, so a numeric literal survives verbatim.
///
/// `project` cannot preserve one: by the time a `Value` exists, `serde_json`
/// has already re-rendered every number. The client reads a response body as
/// text, so it can call this instead.
///
/// # Errors
///
/// [`JsonError`] when the payload is malformed or breaches a parse bound.
pub fn project_raw(
    integration: Integration,
    op: Op,
    remote_json: &str,
) -> Result<String, JsonError> {
    let node = json::parse(remote_json, &Limits::default())?;
    let text = |pointer: &str| {
        node.at(pointer).and_then(json::Node::as_text).unwrap_or("")
    };
    Ok(match (integration, op) {
        (Integration::Jira, Op::Updated) => text("/fields/updated").to_owned(),
        (Integration::Jira, Op::Body) => {
            let summary = text("/fields/summary");
            let description = node
                .at("/fields/description")
                .map_or_else(|| "null".to_owned(), json::Node::canonical);
            format!("{summary}\n{description}")
        }
        (Integration::Linear, Op::Updated) => {
            text("/data/issue/updatedAt").to_owned()
        }
        (Integration::Linear, Op::Body) => {
            let title = text("/data/issue/title");
            let description = text("/data/issue/description");
            format!("{title}\n{description}")
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedIssue<'a> {
    pub title: &'a str,
    pub description: &'a str,
}

/// The inverse of the [`Op::Body`] projection. The title is taken by
/// position, so a description that repeats the title keeps its own line.
#[must_use]
pub fn split_projected(projected: &str) -> ProjectedIssue<'_> {
    projected.split_once('\n').map_or(
        ProjectedIssue {
            title: projected,
            description: "",
        },
        |(title, description)| ProjectedIssue { title, description },
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_integration, project, split_projected, Integration, Op,
        ProjectedIssue,
    };

    #[test]
    fn jira_updated() {
        let remote =
            json!({"fields": {"updated": "2026-01-01T00:00:00.000+0000"}});
        assert_eq!(
            project(Integration::Jira, Op::Updated, &remote),
            "2026-01-01T00:00:00.000+0000"
        );
    }

    #[test]
    fn jira_body_canonicalises_the_adf_description() {
        let remote = json!({
            "fields": {
                "summary": "Test summary",
                "description": {
                    "type": "doc",
                    "version": 1,
                    "content": [{"type": "paragraph", "content": [{"type": "text", "text": "hello"}]}]
                }
            }
        });
        let projected = project(Integration::Jira, Op::Body, &remote);
        assert_eq!(
            projected,
            "Test summary\n{\"content\":[{\"content\":[{\"text\":\"hello\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\"type\":\"doc\",\"version\":1}"
        );
    }

    #[test]
    fn jira_body_canonicalisation_is_independent_of_key_order() {
        let a = json!({
            "fields": {"summary": "S", "description": {"type": "doc", "version": 1}}
        });
        let b = json!({
            "fields": {"summary": "S", "description": {"version": 1, "type": "doc"}}
        });
        assert_eq!(
            project(Integration::Jira, Op::Body, &a),
            project(Integration::Jira, Op::Body, &b)
        );
    }

    #[test]
    fn linear_updated_and_body_no_canonicalisation() {
        let remote = json!({
            "data": {"issue": {"updatedAt": "2026-01-01T00:00:00.000Z", "title": "Test title", "description": "Some *markdown*"}}
        });
        assert_eq!(
            project(Integration::Linear, Op::Updated, &remote),
            "2026-01-01T00:00:00.000Z"
        );
        assert_eq!(
            project(Integration::Linear, Op::Body, &remote),
            "Test title\nSome *markdown*"
        );
    }

    #[test]
    fn missing_fields_are_empty_not_errors() {
        let remote = json!({});
        assert_eq!(project(Integration::Jira, Op::Updated, &remote), "");
        assert_eq!(project(Integration::Jira, Op::Body, &remote), "\nnull");
    }

    #[test]
    fn parse_integration_accepts_only_known_values() {
        assert_eq!(parse_integration("jira"), Some(Integration::Jira));
        assert_eq!(parse_integration("linear"), Some(Integration::Linear));
        assert_eq!(parse_integration("bogus"), None);
    }

    #[test]
    fn splitting_takes_the_first_line_as_the_title() {
        assert_eq!(
            split_projected("Title\nRemote body\n"),
            ProjectedIssue {
                title: "Title",
                description: "Remote body\n",
            }
        );
    }

    #[test]
    fn an_empty_description_splits_to_an_empty_body() {
        assert_eq!(split_projected("Title\n").description, "");
    }

    #[test]
    fn a_description_whose_first_line_equals_the_title_keeps_that_line() {
        assert_eq!(
            split_projected("Title\nTitle\nRemote body").description,
            "Title\nRemote body"
        );
    }

    #[test]
    fn splitting_preserves_the_description_verbatim_including_blank_lines() {
        let description = "\n# Heading\n\n\nparagraph  \n\n";
        assert_eq!(
            split_projected(&format!("Title\n{description}")).description,
            description
        );
    }

    #[test]
    fn a_projection_without_a_newline_is_all_title() {
        assert_eq!(
            split_projected("Title"),
            ProjectedIssue {
                title: "Title",
                description: "",
            }
        );
    }
}
