//! The block-token stream to ADF.
//!
//! Marks cannot nest except inside link text, which recurses and then appends
//! the link mark to every child — giving the array order `[inner…, link]`.
//! Adjacent text nodes are never merged, so `a*b` yields three of them.

use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use crate::adf::tokenise::Token;

/// Assembles a `doc` from a token stream.
///
/// `seed` selects the localid form: `Some` gives the deterministic
/// `00000000-0000-4000-8000-00000000000N` form, `None` the bare counter as a
/// string.
#[must_use]
pub fn to_document(tokens: &[Token], seed: Option<&str>) -> Value {
    let mut state = Assembler::new(seed);
    for token in tokens {
        state.process(token);
    }
    state.flush_all();
    json!({"version": 1, "type": "doc", "content": state.nodes})
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
    Task,
}

struct Assembler<'a> {
    nodes: Vec<Value>,
    paragraph: Option<Vec<Value>>,
    previous_hard_break: bool,
    list_kind: Option<ListKind>,
    list_items: Vec<Value>,
    code_open: bool,
    code_language: String,
    code_lines: Vec<String>,
    local_ids: u64,
    seed: Option<&'a str>,
}

impl<'a> Assembler<'a> {
    const fn new(seed: Option<&'a str>) -> Self {
        Self {
            nodes: Vec::new(),
            paragraph: None,
            previous_hard_break: false,
            list_kind: None,
            list_items: Vec::new(),
            code_open: false,
            code_language: String::new(),
            code_lines: Vec::new(),
            local_ids: 0,
            seed,
        }
    }

    fn next_local_id(&mut self) -> String {
        self.local_ids += 1;
        if self.seed.is_some() {
            format!("00000000-0000-4000-8000-{:012}", self.local_ids)
        } else {
            self.local_ids.to_string()
        }
    }

    fn flush_paragraph(&mut self) {
        if let Some(content) = self.paragraph.take() {
            self.nodes
                .push(json!({"type": "paragraph", "content": content}));
            self.previous_hard_break = false;
        }
    }

    fn flush_list(&mut self) {
        let Some(kind) = self.list_kind.take() else {
            return;
        };
        let items = std::mem::take(&mut self.list_items);
        let node = match kind {
            ListKind::Bullet => json!({"type": "bulletList", "content": items}),
            ListKind::Ordered => json!({
                "type": "orderedList",
                "attrs": {"order": 1},
                "content": items
            }),
            ListKind::Task => {
                let local_id = self.next_local_id();
                json!({
                    "type": "taskList",
                    "attrs": {"localId": local_id},
                    "content": items
                })
            }
        };
        self.nodes.push(node);
    }

    fn flush_code(&mut self) {
        if !self.code_open {
            return;
        }
        let body = std::mem::take(&mut self.code_lines).join("\n");
        let language = std::mem::take(&mut self.code_language);
        self.nodes.push(json!({
            "type": "codeBlock",
            "attrs": {"language": language},
            "content": [{"type": "text", "text": body}]
        }));
        self.code_open = false;
    }

    fn flush_all(&mut self) {
        self.flush_paragraph();
        self.flush_list();
        self.flush_code();
    }

    fn open_list(&mut self, kind: ListKind) {
        if self.list_kind != Some(kind) {
            self.flush_list();
            self.list_kind = Some(kind);
        }
    }

    fn process(&mut self, token: &Token) {
        match token {
            Token::Paragraph(text) => {
                let inlines = parse_inlines(text);
                if self.previous_hard_break {
                    if let Some(paragraph) = self.paragraph.as_mut() {
                        paragraph.extend(inlines);
                    }
                } else {
                    self.flush_paragraph();
                    self.flush_list();
                    self.paragraph = Some(inlines);
                }
                self.previous_hard_break = false;
            }
            Token::HardBreak => {
                if let Some(paragraph) = self.paragraph.as_mut() {
                    paragraph.push(json!({"type": "hardBreak"}));
                    self.previous_hard_break = true;
                }
            }
            Token::Heading { level, text } => {
                self.flush_paragraph();
                self.flush_list();
                self.nodes.push(json!({
                    "type": "heading",
                    "attrs": {"level": level},
                    "content": parse_inlines(text)
                }));
            }
            Token::Bullet(text) => {
                self.flush_paragraph();
                self.open_list(ListKind::Bullet);
                self.push_list_item(text);
            }
            Token::Ordered { text, .. } => {
                self.flush_paragraph();
                self.open_list(ListKind::Ordered);
                self.push_list_item(text);
            }
            Token::TaskTodo(text) | Token::TaskDone(text) => {
                self.flush_paragraph();
                self.open_list(ListKind::Task);
                let local_id = self.next_local_id();
                let state = if matches!(token, Token::TaskDone(_)) {
                    "DONE"
                } else {
                    "TODO"
                };
                self.list_items.push(json!({
                    "type": "taskItem",
                    "attrs": {"localId": local_id, "state": state},
                    "content": parse_inlines(text)
                }));
            }
            Token::CodeOpen(language) => {
                self.flush_paragraph();
                self.flush_list();
                self.code_open = true;
                self.code_language.clone_from(language);
                self.code_lines.clear();
            }
            Token::CodeLine(line) => {
                if self.code_open {
                    self.code_lines.push(line.clone());
                }
            }
            Token::CodeClose => self.flush_code(),
        }
    }

    fn push_list_item(&mut self, text: &str) {
        self.list_items.push(json!({
            "type": "listItem",
            "content": [{"type": "paragraph", "content": parse_inlines(text)}]
        }));
    }
}

/// The inline scan, in the alternation's own priority order: code span,
/// `***`, `**`, `*`, `[text](url)`, bare `[text]`, a plain run, then a
/// single-character catch-all.
///
/// The catch-all is why an unbalanced `*` becomes an em-marked **empty**
/// string rather than a literal asterisk: the token is `*`, the classifier
/// sees a leading `*`, and stripping one delimiter from each end of a
/// one-character token leaves nothing.
#[must_use]
pub fn parse_inlines(text: &str) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let (token, remainder) = next_token(rest);
        nodes.extend(classify(token));
        rest = remainder;
    }
    nodes
}

fn next_token(text: &str) -> (&str, &str) {
    let candidates = [
        delimited(text, "`", "`", |inner| !inner.contains('`')),
        delimited(text, "***", "***", |inner| !inner.contains('*')),
        delimited(text, "**", "**", |inner| !inner.contains('*')),
        delimited(text, "*", "*", |inner| !inner.contains('*')),
        link_token(text),
        bracket_token(text),
        plain_run(text),
    ];
    if let Some(length) = candidates.into_iter().flatten().next() {
        return text.split_at(length);
    }
    let width = text.chars().next().map_or(1, char::len_utf8);
    text.split_at(width)
}

/// `<open><inner><close>` where `inner` is non-empty and satisfies `admissible`.
fn delimited(
    text: &str,
    open: &str,
    close: &str,
    admissible: impl Fn(&str) -> bool,
) -> Option<usize> {
    let after_open = text.strip_prefix(open)?;
    let end = after_open.find(close)?;
    let inner = &after_open[..end];
    if inner.is_empty() || !admissible(inner) {
        return None;
    }
    Some(open.len() + inner.len() + close.len())
}

/// `\[[^\]]*\]\([^)]+\)`
fn link_token(text: &str) -> Option<usize> {
    let after_open = text.strip_prefix('[')?;
    let close = after_open.find(']')?;
    let after_text = after_open.get(close + 1..)?.strip_prefix('(')?;
    let end = after_text.find(')')?;
    if end == 0 {
        return None;
    }
    Some(1 + close + 2 + end + 1)
}

/// `\[[^\]]*\]`
fn bracket_token(text: &str) -> Option<usize> {
    let after_open = text.strip_prefix('[')?;
    let close = after_open.find(']')?;
    Some(close + 2)
}

/// A run of characters that are none of a backtick, an asterisk or `[`.
fn plain_run(text: &str) -> Option<usize> {
    let length = text.find(['`', '*', '[']).unwrap_or(text.len());
    (length > 0).then_some(length)
}

fn text_node(text: &str, marks: &[Value]) -> Value {
    let mut node = Map::new();
    node.insert("type".to_owned(), json!("text"));
    node.insert("text".to_owned(), json!(text));
    if !marks.is_empty() {
        node.insert("marks".to_owned(), json!(marks));
    }
    Value::Object(node)
}

fn classify(token: &str) -> Vec<Value> {
    if let Some(inner) = strip_pair(token, "`") {
        return vec![text_node(inner, &[json!({"type": "code"})])];
    }
    if let Some(inner) = strip_pair(token, "***") {
        return vec![text_node(
            inner,
            &[json!({"type": "strong"}), json!({"type": "em"})],
        )];
    }
    if let Some(inner) = strip_pair(token, "**") {
        return vec![text_node(inner, &[json!({"type": "strong"})])];
    }
    if let Some(inner) = strip_pair(token, "*") {
        return vec![text_node(inner, &[json!({"type": "em"})])];
    }
    if let Some((label, href)) = link_parts(token) {
        // A link requires a non-empty label; a token with an empty label is
        // silently dropped rather than emitted as text.
        if label.is_empty() {
            return Vec::new();
        }
        let mark = json!({"type": "link", "attrs": {"href": href}});
        return parse_inlines(label)
            .into_iter()
            .map(|node| with_mark(node, &mark))
            .collect();
    }
    vec![text_node(token, &[])]
}

/// Strips one delimiter from each end, so a one-character token yields an
/// empty string.
fn strip_pair<'a>(token: &'a str, delimiter: &str) -> Option<&'a str> {
    if !token.starts_with(delimiter) {
        return None;
    }
    let width = delimiter.len();
    let start = width.min(token.len());
    let end = token.len().saturating_sub(width).max(start);
    Some(&token[start..end])
}

fn link_parts(token: &str) -> Option<(&str, &str)> {
    let after_open = token.strip_prefix('[')?;
    let close = after_open.find("](")?;
    let label = &after_open[..close];
    let href = after_open.get(close + 2..)?.strip_suffix(')')?;
    Some((label, href))
}

fn with_mark(node: Value, mark: &Value) -> Value {
    let Value::Object(mut fields) = node else {
        return node;
    };
    let mut marks = fields
        .get("marks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    marks.push(mark.clone());
    fields.insert("marks".to_owned(), Value::Array(marks));
    Value::Object(fields)
}
