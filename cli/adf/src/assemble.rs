//! The block-token stream to ADF.
//!
//! Emphasis, strike and link text recurse, then append their own marks to
//! every child — giving the array order `[inner…, outer]`. Adjacent text
//! nodes are never merged, so `a*b` yields three of them.

use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use crate::syntax;
use crate::tokenise::Token;

/// Assembles a `doc` from a token stream.
///
/// `seed` selects the localid form: `Some` gives the deterministic
/// `00000000-0000-4000-8000-00000000000N` form, `None` the bare counter as a
/// string.
#[must_use]
pub fn to_document(tokens: &[Token], seed: Option<&str>) -> Value {
    let mut local_ids = LocalIds {
        issued: 0,
        seeded: seed.is_some(),
    };
    json!({"version": 1, "type": "doc", "content": blocks(tokens, &mut local_ids)})
}

struct LocalIds {
    issued: u64,
    seeded: bool,
}

impl LocalIds {
    fn next(&mut self) -> String {
        self.issued += 1;
        if self.seeded {
            format!("00000000-0000-4000-8000-{:012}", self.issued)
        } else {
            self.issued.to_string()
        }
    }
}

fn blocks(tokens: &[Token], local_ids: &mut LocalIds) -> Vec<Value> {
    let mut assembler = Assembler::new(local_ids);
    for token in tokens {
        assembler.process(token);
    }
    assembler.flush_all();
    assembler.nodes
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
    Task,
}

struct Assembler<'a> {
    local_ids: &'a mut LocalIds,
    nodes: Vec<Value>,
    paragraph: Option<Vec<Value>>,
    previous_hard_break: bool,
    list_kind: Option<ListKind>,
    list_start: u64,
    list_items: Vec<Value>,
    code_open: bool,
    code_language: String,
    code_lines: Vec<String>,
}

impl<'a> Assembler<'a> {
    const fn new(local_ids: &'a mut LocalIds) -> Self {
        Self {
            local_ids,
            nodes: Vec::new(),
            paragraph: None,
            previous_hard_break: false,
            list_kind: None,
            list_start: 1,
            list_items: Vec::new(),
            code_open: false,
            code_language: String::new(),
            code_lines: Vec::new(),
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
                "attrs": {"order": self.list_start},
                "content": items
            }),
            ListKind::Task => {
                let local_id = self.local_ids.next();
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

    fn start_block(&mut self, node: Value) {
        self.flush_paragraph();
        self.flush_list();
        self.nodes.push(node);
    }

    fn open_list(&mut self, kind: ListKind) -> bool {
        if self.list_kind == Some(kind) {
            return false;
        }
        self.flush_list();
        self.list_kind = Some(kind);
        true
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
            Token::Heading { level, text } => self.start_block(json!({
                "type": "heading",
                "attrs": {"level": level},
                "content": parse_inlines(text)
            })),
            Token::Bullet(item) => {
                self.flush_paragraph();
                self.open_list(ListKind::Bullet);
                self.push_list_item(item);
            }
            Token::Ordered { number, item } => {
                self.flush_paragraph();
                if self.open_list(ListKind::Ordered) {
                    self.list_start = number.parse().unwrap_or(1);
                }
                self.push_list_item(item);
            }
            Token::TaskTodo(text) | Token::TaskDone(text) => {
                self.flush_paragraph();
                self.open_list(ListKind::Task);
                let local_id = self.local_ids.next();
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
            Token::BlockQuote(content) => {
                let content = blocks(content, self.local_ids);
                self.start_block(
                    json!({"type": "blockquote", "content": content}),
                );
            }
            Token::Table(rows) => self.start_block(table(rows)),
            Token::Rule => self.start_block(json!({"type": "rule"})),
        }
    }

    fn push_list_item(&mut self, item: &[Token]) {
        let content = blocks(item, self.local_ids);
        self.list_items
            .push(json!({"type": "listItem", "content": content}));
    }
}

/// The first row's cells are headers; every cell holds one paragraph.
fn table(rows: &[Vec<String>]) -> Value {
    let rows: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(index, cells)| {
            let kind = if index == 0 { "tableHeader" } else { "tableCell" };
            let cells: Vec<Value> = cells
                .iter()
                .map(|cell| {
                    json!({
                        "type": kind,
                        "content": [{"type": "paragraph", "content": parse_inlines(cell)}]
                    })
                })
                .collect();
            json!({"type": "tableRow", "content": cells})
        })
        .collect();
    json!({"type": "table", "content": rows})
}

/// The inline scan, in the alternation's own priority order: code span,
/// `***`, `**`, `*`, `~~`, `[text](url)`, bare `[text]`, a plain run, then a
/// single-character catch-all. Every scan but the code span's skips a
/// backslash-escaped character.
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
        code_span(text),
        delimited(text, "***", |inner| !contains_unescaped(inner, "*")),
        delimited(text, "**", |inner| !contains_unescaped(inner, "*")),
        delimited(text, "*", |inner| !contains_unescaped(inner, "*")),
        delimited(text, syntax::STRIKE, |_| true),
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

fn contains_unescaped(text: &str, pattern: &str) -> bool {
    syntax::find_unescaped(text, pattern).is_some()
}

/// `` `[^`]+` `` — a backslash escapes nothing inside a code span.
fn code_span(text: &str) -> Option<usize> {
    let after_open = text.strip_prefix('`')?;
    let end = after_open.find('`')?;
    (end > 0).then_some(end + 2)
}

/// `<delimiter><inner><delimiter>` where `inner` is non-empty and satisfies
/// `admissible`.
fn delimited(
    text: &str,
    delimiter: &str,
    admissible: impl Fn(&str) -> bool,
) -> Option<usize> {
    let after_open = text.strip_prefix(delimiter)?;
    let end = syntax::find_unescaped(after_open, delimiter)?;
    let inner = &after_open[..end];
    if inner.is_empty() || !admissible(inner) {
        return None;
    }
    Some(2 * delimiter.len() + inner.len())
}

/// `\[[^\]]*\]\([^)]+\)`
fn link_token(text: &str) -> Option<usize> {
    let (label, href) = link_parts(text)?;
    if href.is_empty() {
        return None;
    }
    Some(label.len() + href.len() + 4)
}

/// `\[[^\]]*\]`
fn bracket_token(text: &str) -> Option<usize> {
    let after_open = text.strip_prefix('[')?;
    let close = syntax::find_unescaped(after_open, "]")?;
    Some(close + 2)
}

/// A run of characters up to the first unescaped backtick, asterisk, `[` or
/// `~~`.
fn plain_run(text: &str) -> Option<usize> {
    let mut characters = text.char_indices().peekable();
    let mut length = 0;
    while let Some((offset, character)) = characters.next() {
        let opens_strike = text[offset..].starts_with(syntax::STRIKE);
        if matches!(character, '`' | '*' | '[') || opens_strike {
            break;
        }
        length = offset + character.len_utf8();
        if character == syntax::ESCAPE {
            if let Some((next_offset, next)) =
                characters.next_if(|(_, next)| syntax::is_escapable(*next))
            {
                length = next_offset + next.len_utf8();
            }
        }
    }
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
    for (delimiter, marks) in [
        ("***", &["strong", "em"][..]),
        ("**", &["strong"][..]),
        ("*", &["em"][..]),
        (syntax::STRIKE, &["strike"][..]),
    ] {
        if let Some(inner) = strip_pair(token, delimiter) {
            return marked(inner, marks);
        }
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
    vec![text_node(&syntax::unescape(token), &[])]
}

/// An empty inner, as the catch-all leaves, still yields one empty node.
fn marked(inner: &str, marks: &[&str]) -> Vec<Value> {
    let children = if inner.is_empty() {
        vec![text_node("", &[])]
    } else {
        parse_inlines(inner)
    };
    children
        .into_iter()
        .map(|node| {
            marks.iter().fold(node, |node, mark| {
                with_mark(node, &json!({"type": mark}))
            })
        })
        .collect()
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
    let close = syntax::find_unescaped(after_open, "]")?;
    let label = &after_open[..close];
    let after_label = after_open[close + 1..].strip_prefix('(')?;
    let end = after_label.find(')')?;
    Some((label, &after_label[..end]))
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
