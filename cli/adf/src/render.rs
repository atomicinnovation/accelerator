//! ADF to Markdown.
//!
//! Untyped `serde_json::Value` end to end: a typed struct would reorder keys
//! and its declaration-order serialisation would silently rehash every Jira
//! item.
//!
//! Byte-fidelity rules that are easy to lose: blocks joined with a blank line
//! and inlines with nothing, an empty `doc.content` producing zero bytes rather
//! than a newline, and a hard break emitted as two spaces then a newline. Text
//! is escaped only where the tokeniser would read it as syntax, so every
//! rendered body reads back as itself.

use serde_json::Value;

use crate::syntax;
use crate::AdfError;
use crate::Unrepresentable;

/// What a node with no Markdown form becomes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NoMarkdownForm {
    Placeholder,
    Refusal,
}

/// Renders a `doc` to the markdown subset, without a trailing newline.
///
/// The reference output has exactly one trailing newline for a non-empty
/// document and none at all for an empty one; callers that need byte parity
/// add it back.
///
/// # Errors
///
/// [`AdfError`] on an unrenderable shape, or on a node with no Markdown form
/// when that is a [`NoMarkdownForm::Refusal`].
pub fn to_markdown(
    document: &Value,
    no_markdown_form: NoMarkdownForm,
) -> Result<String, AdfError> {
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
    Renderer { no_markdown_form }.blocks(&blocks)
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

fn is_list(node: &Value) -> bool {
    matches!(node_type(node), "bulletList" | "orderedList" | "taskList")
}

fn lines_prefixed(body: &str, first: &str, rest: &str) -> String {
    body.split('\n')
        .enumerate()
        .map(|(index, line)| {
            let prefix = if index == 0 { first } else { rest };
            if line.is_empty() {
                prefix.trim_end().to_owned()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where a run of text sits, which decides the characters escaped beyond the
/// inline markers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Surroundings {
    Prose,
    TableCell,
}

struct Renderer {
    no_markdown_form: NoMarkdownForm,
}

impl Renderer {
    fn admit_unrepresentable(
        &self,
        reason: Unrepresentable,
    ) -> Result<(), AdfError> {
        match self.no_markdown_form {
            NoMarkdownForm::Placeholder => Ok(()),
            NoMarkdownForm::Refusal => Err(AdfError::Unrepresentable(reason)),
        }
    }

    fn block_placeholder(&self, node: &str) -> Result<String, AdfError> {
        self.block_placeholder_because(node, unsupported(node))
    }

    fn block_placeholder_because(
        &self,
        node: &str,
        reason: Unrepresentable,
    ) -> Result<String, AdfError> {
        self.admit_unrepresentable(reason)?;
        Ok(format!("[unsupported ADF node: {node}]"))
    }

    fn inline_placeholder(&self, node: &str) -> Result<String, AdfError> {
        self.inline_placeholder_because(node, unsupported(node))
    }

    fn inline_placeholder_because(
        &self,
        node: &str,
        reason: Unrepresentable,
    ) -> Result<String, AdfError> {
        self.admit_unrepresentable(reason)?;
        Ok(format!("[unsupported ADF inline: {node}]"))
    }

    /// Blocks that render to nothing leave no blank line behind.
    fn blocks(&self, blocks: &[Value]) -> Result<String, AdfError> {
        let mut rendered = Vec::with_capacity(blocks.len());
        for block in blocks {
            let block = self.block(block)?;
            if !block.is_empty() {
                rendered.push(block);
            }
        }
        Ok(rendered.join("\n\n"))
    }

    fn block(&self, block: &Value) -> Result<String, AdfError> {
        match node_type(block) {
            "paragraph" => self.paragraph(block),
            "heading" => {
                let level = attribute(block, "level")
                    .and_then(Value::as_u64)
                    .ok_or(AdfError::HeadingWithoutLevel)?;
                if !(1..=6).contains(&level) {
                    self.admit_unrepresentable(
                        Unrepresentable::HeadingBeyondLevelSix,
                    )?;
                }
                let hashes = "#".repeat(usize::try_from(level).unwrap_or(0));
                let text =
                    self.inlines(&children(block), Surroundings::Prose)?;
                Ok(format!("{hashes} {text}"))
            }
            "bulletList" => {
                let items = required_children(block, "bulletList")?;
                self.list_items(items, |_| "- ".to_owned())
            }
            "orderedList" => {
                let start = attribute(block, "order")
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let items = required_children(block, "orderedList")?;
                self.list_items(items, |index| {
                    format!("{}. ", start.saturating_add(index as u64))
                })
            }
            "taskList" => {
                let items = required_children(block, "taskList")?;
                let mut rendered = Vec::with_capacity(items.len());
                for item in items {
                    let marker = if attribute(item, "state")
                        .and_then(Value::as_str)
                        == Some("DONE")
                    {
                        "- [x] "
                    } else {
                        "- [ ] "
                    };
                    let text =
                        self.inlines(&children(item), Surroundings::Prose)?;
                    rendered.push(format!("{marker}{text}"));
                }
                Ok(rendered.join("\n"))
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
                if body.lines().any(|line| line.starts_with(syntax::FENCE)) {
                    self.admit_unrepresentable(
                        Unrepresentable::FenceLineInCodeBlock,
                    )?;
                }
                Ok(format!("```{language}\n{body}\n```"))
            }
            "blockquote" => {
                let body = self.blocks(&children(block))?;
                if body.is_empty() {
                    return Ok(body);
                }
                Ok(lines_prefixed(&body, "> ", "> "))
            }
            "table" => self.table(block),
            "rule" => Ok(syntax::RULE.to_owned()),
            other => self.block_placeholder(other),
        }
    }

    fn paragraph(&self, paragraph: &Value) -> Result<String, AdfError> {
        let text = self.inlines(&children(paragraph), Surroundings::Prose)?;
        Ok(text
            .split('\n')
            .map(syntax::escape_line_start)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn list_items(
        &self,
        items: &[Value],
        marker: impl Fn(usize) -> String,
    ) -> Result<String, AdfError> {
        let mut rendered = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            rendered.push(self.list_item(item, &marker(index))?);
        }
        Ok(rendered.join("\n"))
    }

    /// The first paragraph follows the marker; every later child is indented
    /// to the marker's width. A nested list needs no blank line before it, and
    /// anything else does, so it reads back as a block of its own.
    fn list_item(
        &self,
        item: &Value,
        marker: &str,
    ) -> Result<String, AdfError> {
        let children = children(item);
        let (lead, rest) = match children.split_first() {
            Some((first, rest)) if node_type(first) == "paragraph" => {
                (self.paragraph(first)?, rest)
            }
            _ => (String::new(), children.as_slice()),
        };
        let indent = " ".repeat(marker.len());

        let mut rendered = if lead.is_empty() {
            marker.to_owned()
        } else {
            lines_prefixed(&lead, marker, &indent)
        };
        for block in rest {
            let body = self.block(block)?;
            if body.is_empty() {
                continue;
            }
            rendered.push_str(if is_list(block) { "\n" } else { "\n\n" });
            rendered.push_str(&lines_prefixed(&body, &indent, &indent));
        }
        Ok(rendered)
    }

    /// The first row is the header, whatever its cells' types.
    fn table(&self, table: &Value) -> Result<String, AdfError> {
        let mut lines = Vec::new();
        for (index, row) in children(table).iter().enumerate() {
            let mut cells = Vec::new();
            for cell in children(row) {
                cells.push(self.cell(&cell)?);
            }
            lines.push(format!("| {} |", cells.join(" | ")));
            if index == 0 {
                lines.push(format!(
                    "| {} |",
                    vec!["---"; cells.len()].join(" | ")
                ));
            }
        }
        Ok(lines.join("\n"))
    }

    fn cell(&self, cell: &Value) -> Result<String, AdfError> {
        match children(cell).as_slice() {
            [] => Ok(String::new()),
            [paragraph] if node_type(paragraph) == "paragraph" => {
                self.inlines(&children(paragraph), Surroundings::TableCell)
            }
            _ => self.block_placeholder_because(
                node_type(cell),
                Unrepresentable::TableCellOfSeveralBlocks,
            ),
        }
    }

    /// Adjacent text nodes carrying the same marks render as one run.
    fn inlines(
        &self,
        inlines: &[Value],
        surroundings: Surroundings,
    ) -> Result<String, AdfError> {
        let mut rendered = String::new();
        let mut run: Option<(Marks, String)> = None;
        let flush = |run: &mut Option<(Marks, String)>,
                     rendered: &mut String| {
            if let Some((marks, text)) = run.take() {
                rendered.push_str(&marks.wrap(&text, surroundings));
            }
        };

        for node in inlines {
            match node_type(node) {
                "text" => {
                    let marks = Marks::of(node);
                    let text =
                        node.get("text").and_then(Value::as_str).unwrap_or("");
                    if marks.code && text.contains('`') {
                        self.admit_unrepresentable(
                            Unrepresentable::BacktickInInlineCode,
                        )?;
                    }
                    if surroundings == Surroundings::TableCell {
                        if marks.code && text.contains('|') {
                            self.admit_unrepresentable(
                                Unrepresentable::PipeInTableCellCode,
                            )?;
                        }
                        if marks
                            .link
                            .as_ref()
                            .is_some_and(|href| href.contains('|'))
                        {
                            self.admit_unrepresentable(
                                Unrepresentable::PipeInTableCellLink,
                            )?;
                        }
                    }
                    match run.as_mut() {
                        Some((open, joined)) if *open == marks => {
                            joined.push_str(text);
                        }
                        _ => {
                            flush(&mut run, &mut rendered);
                            run = Some((marks, text.to_owned()));
                        }
                    }
                }
                "hardBreak" if surroundings == Surroundings::TableCell => {
                    flush(&mut run, &mut rendered);
                    rendered.push_str(&self.inline_placeholder_because(
                        "hardBreak",
                        Unrepresentable::HardBreakInTableCell,
                    )?);
                }
                "hardBreak" => {
                    flush(&mut run, &mut rendered);
                    rendered.push_str("  \n");
                }
                other => {
                    flush(&mut run, &mut rendered);
                    rendered.push_str(&self.inline_placeholder(other)?);
                }
            }
        }
        flush(&mut run, &mut rendered);
        Ok(rendered)
    }
}

/// The marks with a Markdown form. Membership decides, not array order, so an
/// ADF node's `marks` order is irrelevant.
#[derive(PartialEq, Eq)]
struct Marks {
    code: bool,
    emphasis: Vec<&'static str>,
    link: Option<String>,
}

impl Marks {
    fn of(node: &Value) -> Self {
        let marks = node
            .get("marks")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, Clone::clone);
        let has = |kind: &str| {
            marks.iter().any(|mark| {
                mark.get("type").and_then(Value::as_str) == Some(kind)
            })
        };
        let link = has("link")
            .then(|| {
                marks
                    .iter()
                    .find(|mark| {
                        mark.get("type").and_then(Value::as_str) == Some("link")
                    })
                    .and_then(|mark| mark.get("attrs"))
                    .and_then(|attrs| attrs.get("href"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            })
            .filter(|href| href_is_safe(href));
        let emphasis = [
            (has("em"), "*"),
            (has("strong"), "**"),
            (has("strike"), syntax::STRIKE),
        ]
        .into_iter()
        .filter_map(|(present, delimiter)| present.then_some(delimiter))
        .collect();
        Self {
            code: has("code"),
            emphasis,
            link,
        }
    }

    /// Applied as a fixed pipeline, innermost first: `code`, `em`, `strong`,
    /// `strike`, then `link`. A run without text renders nothing, since empty
    /// delimiters would not read back as the same node.
    ///
    /// The emitted href is the untrimmed original: only the scheme check
    /// trims, so leading whitespace survives into the output.
    fn wrap(&self, text: &str, surroundings: Surroundings) -> String {
        if text.is_empty() {
            return String::new();
        }
        let mut also = Vec::new();
        if surroundings == Surroundings::TableCell {
            also.push('|');
        }
        if self.link.is_some() {
            also.push(']');
        }

        let mut rendered = if self.code {
            format!("`{text}`")
        } else {
            syntax::escape_inline(text, &also)
        };
        for delimiter in &self.emphasis {
            rendered = format!("{delimiter}{rendered}{delimiter}");
        }
        match &self.link {
            Some(href) => format!("[{rendered}]({href})"),
            None => rendered,
        }
    }
}

fn unsupported(node: &str) -> Unrepresentable {
    Unrepresentable::Node {
        node: node.to_owned(),
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
