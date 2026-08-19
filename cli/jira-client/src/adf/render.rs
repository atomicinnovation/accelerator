//! ADF to Markdown.
//!
//! Untyped `serde_json::Value` end to end: a typed struct would reorder keys
//! and its declaration-order serialisation would silently rehash every Jira
//! item.
//!
//! Byte-fidelity rules that are easy to lose: no indentation anywhere (nested
//! structure is unrepresentable), no text escaping at all, blocks joined with
//! a blank line and inlines with nothing, an empty `doc.content` producing
//! zero bytes rather than a newline, and a hard break emitted as two spaces
//! then a newline.

use serde_json::Value;

use crate::adf::AdfError;

/// Renders a `doc` to the markdown subset, without a trailing newline.
///
/// The reference output has exactly one trailing newline for a non-empty
/// document and none at all for an empty one; callers that need byte parity
/// add it back.
///
/// # Errors
///
/// [`AdfError`] on an unrenderable shape.
pub fn to_markdown(document: &Value) -> Result<String, AdfError> {
    let kind = document.get("type").and_then(Value::as_str);
    if kind != Some("doc") {
        return Err(AdfError::RootNotDoc {
            found: kind.unwrap_or("null").to_owned(),
        });
    }
    let Some(blocks) = document.get("content") else {
        return Ok(String::new());
    };
    let blocks = blocks.as_array().map_or_else(Vec::new, Clone::clone);
    let mut rendered = Vec::with_capacity(blocks.len());
    for block in &blocks {
        rendered.push(render_block(block)?);
    }
    Ok(rendered.join("\n\n"))
}

fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("null")
}

fn attribute<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node.get("attrs").and_then(|attrs| attrs.get(name))
}

/// The children of a node that must have them: `bulletList`, `orderedList`
/// and `taskList` read `.content` with no `// []` fallback, so an absent one
/// aborts the whole document rather than rendering as empty.
fn required_children<'a>(
    node: &'a Value,
    kind: &str,
) -> Result<&'a Vec<Value>, AdfError> {
    node.get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| AdfError::ListWithoutContent {
            node: kind.to_owned(),
        })
}

fn children(node: &Value) -> Vec<Value> {
    node.get("content")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone)
}

fn render_block(block: &Value) -> Result<String, AdfError> {
    match node_type(block) {
        "paragraph" => Ok(render_inlines(&children(block))),
        "heading" => {
            let level = attribute(block, "level")
                .and_then(Value::as_u64)
                .ok_or(AdfError::HeadingWithoutLevel)?;
            let hashes = "#".repeat(usize::try_from(level).unwrap_or(0));
            Ok(format!("{hashes} {}", render_inlines(&children(block))))
        }
        "bulletList" => {
            let items = required_children(block, "bulletList")?;
            Ok(items
                .iter()
                .map(|item| format!("- {}", list_item_text(item)))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "orderedList" => {
            let start = attribute(block, "order")
                .and_then(Value::as_u64)
                .unwrap_or(1);
            let items = required_children(block, "orderedList")?;
            Ok(items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let number = start.saturating_add(index as u64);
                    format!("{number}. {}", list_item_text(item))
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "taskList" => {
            let items = required_children(block, "taskList")?;
            Ok(items
                .iter()
                .map(|item| {
                    let marker = if attribute(item, "state")
                        .and_then(Value::as_str)
                        == Some("DONE")
                    {
                        "- [x] "
                    } else {
                        "- [ ] "
                    };
                    format!("{marker}{}", render_inlines(&children(item)))
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "codeBlock" => {
            let language = attribute(block, "language")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let body = block
                .get("content")
                .and_then(|content| content.get(0))
                .and_then(|first| first.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            Ok(format!("```{language}\n{body}\n```"))
        }
        other => Ok(format!("[unsupported ADF node: {other}]")),
    }
}

/// A list item renders its **first** child's content only. A second paragraph
/// or a nested list under the same item is dropped without a placeholder;
/// a nested list as the *first* child renders one inline placeholder per
/// `listItem`, because its children are list items rather than inlines.
fn list_item_text(item: &Value) -> String {
    let inlines = item
        .get("content")
        .and_then(|content| content.get(0))
        .map(children)
        .unwrap_or_default();
    render_inlines(&inlines)
}

fn render_inlines(inlines: &[Value]) -> String {
    inlines.iter().map(render_inline).collect()
}

fn render_inline(node: &Value) -> String {
    match node_type(node) {
        "text" => render_text(node),
        "hardBreak" => "  \n".to_owned(),
        other => format!("[unsupported ADF inline: {other}]"),
    }
}

/// Marks are applied as a fixed pipeline, innermost first: `code`, `em`,
/// `strong`, then `link`. Membership decides, not array order, so an ADF
/// node's `marks` order is irrelevant and the nesting is always this one.
///
/// A text node with no `.text` is not an error: the missing text is treated
/// as an empty string, leaving any surrounding delimiters in place.
fn render_text(node: &Value) -> String {
    let marks = node
        .get("marks")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone);
    let has = |kind: &str| {
        marks
            .iter()
            .any(|mark| mark.get("type").and_then(Value::as_str) == Some(kind))
    };

    let mut rendered = node
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if has("code") {
        rendered = format!("`{rendered}`");
    }
    if has("em") {
        rendered = format!("*{rendered}*");
    }
    if has("strong") {
        rendered = format!("**{rendered}**");
    }
    if !has("link") {
        return rendered;
    }
    let href = marks
        .iter()
        .find(|mark| mark.get("type").and_then(Value::as_str) == Some("link"))
        .and_then(|mark| mark.get("attrs"))
        .and_then(|attrs| attrs.get("href"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if href_is_safe(href) {
        // The emitted href is the UNTRIMMED original: only the scheme check
        // trims, so leading whitespace survives into the output.
        format!("[{rendered}]({href})")
    } else {
        rendered
    }
}

/// Allows `http`, `https` and `mailto`, plus anything schemeless — relative,
/// fragment or protocol-relative. Any other scheme drops the mark and leaves
/// the text bare, with no placeholder.
fn href_is_safe(href: &str) -> bool {
    let trimmed = href.trim_start_matches([' ', '\t', '\n', '\r']);
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
    {
        return true;
    }
    !has_scheme(&lower)
}

/// `^[a-z][a-z0-9+.\-]*:` over the lowercased href.
fn has_scheme(lower: &str) -> bool {
    let mut characters = lower.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    for character in characters {
        match character {
            ':' => return true,
            'a'..='z' | '0'..='9' | '+' | '.' | '-' => {}
            _ => return false,
        }
    }
    false
}
