//! Jira stores descriptions as ADF, so its bodies differ from the Markdown a
//! local work item holds.

use std::borrow::Cow;

use serde_json::Value;
use tracker::BodyDialect;
use tracker::Unrenderable;

/// The Jira body rules, shared by [`crate::JiraClient`] and the offline test
/// doubles so both answer through one implementation.
pub struct JiraBodyDialect;

/// The projection's spelling of a Jira issue with no description.
const ABSENT_DESCRIPTION: &str = "null";

fn with_terminating_newline(markdown: String) -> String {
    if markdown.is_empty() {
        markdown
    } else {
        markdown + "\n"
    }
}

fn unrenderable(detail: impl std::fmt::Display) -> Unrenderable {
    Unrenderable {
        detail: detail.to_string(),
    }
}

impl BodyDialect for JiraBodyDialect {
    fn local_body(&self, description: &str) -> Result<String, Unrenderable> {
        let description = description.trim_end();
        if description == ABSENT_DESCRIPTION {
            return Ok(String::new());
        }
        let document: Value =
            serde_json::from_str(description).map_err(unrenderable)?;
        adf::document_to_markdown_strict(&document)
            .map(with_terminating_newline)
            .map_err(unrenderable)
    }

    fn as_pushed<'a>(&self, body: &'a str) -> Cow<'a, str> {
        adf::markdown_to_document(body, None)
            .and_then(|document| adf::document_to_markdown(&document))
            .map_or(Cow::Borrowed(body), |markdown| {
                Cow::Owned(with_terminating_newline(markdown))
            })
    }

    fn is_wire_document(&self, body: &str) -> bool {
        serde_json::from_str::<Value>(body.trim()).is_ok_and(|document| {
            document.get("type").and_then(Value::as_str) == Some("doc")
        })
    }
}

#[cfg(test)]
mod tests {
    use tracker::BodyDialect;

    use super::JiraBodyDialect;

    #[test]
    fn an_adf_doc_is_a_wire_document() {
        assert!(JiraBodyDialect.is_wire_document(
            "{\"content\":[],\"type\":\"doc\",\"version\":1}\n"
        ));
    }

    #[test]
    fn markdown_is_not_a_wire_document() {
        assert!(!JiraBodyDialect.is_wire_document("# Heading\n\nText\n"));
        assert!(!JiraBodyDialect.is_wire_document("{\"type\":\"paragraph\"}"));
    }
}
