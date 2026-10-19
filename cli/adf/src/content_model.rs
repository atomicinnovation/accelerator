//! The nesting Jira's ADF schema allows: what a `blockquote` or a `listItem`
//! may hold. Every other container's content is fixed by assembly itself.

use serde_json::Value;

use crate::AdfError;

const QUOTABLE: &[&str] = &[
    "paragraph",
    "bulletList",
    "orderedList",
    "codeBlock",
    "mediaGroup",
    "mediaSingle",
];

const LISTABLE: &[&str] = &[
    "paragraph",
    "bulletList",
    "orderedList",
    "codeBlock",
    "mediaSingle",
];

/// Admits `document` when every block sits in a container Jira allows it in.
///
/// # Errors
///
/// [`AdfError::DisallowedNesting`] at the first block outside its
/// container's allowed set.
pub fn admit(document: &Value) -> Result<(), AdfError> {
    let container = node_type(document);
    let allowed = match container {
        "blockquote" => Some(QUOTABLE),
        "listItem" => Some(LISTABLE),
        _ => None,
    };
    for child in children(document) {
        if let Some(allowed) = allowed {
            let child_type = node_type(child);
            if !allowed.contains(&child_type) {
                return Err(AdfError::DisallowedNesting {
                    child: child_type.to_owned(),
                    container: container.to_owned(),
                });
            }
        }
        admit(child)?;
    }
    Ok(())
}

fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn children(node: &Value) -> &[Value] {
    node.get("content")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}
