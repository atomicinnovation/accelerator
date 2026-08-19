//! A bounded JSON reader that keeps every number's source token.
//!
//! `serde_json` parses a number into `f64`/`i64`/`u64` and re-renders it, so a
//! literal outside those ranges loses precision and a trailing zero
//! disappears: `1.500` becomes `1.5`. `RemoteIssue.body` feeds `remote_hash`,
//! so a formatting difference on a single numeric custom field would
//! mass-reclassify every such item as remotely modified on the first live
//! sync.
//!
//! Enabling `serde_json/arbitrary_precision` would fix it globally and is
//! forbidden: it changes `Value::Number`'s representation and its
//! `untagged`/`flatten` behaviour for every other crate in the workspace,
//! including the binary that verifies signed artefacts. This reader is local to
//! the projection instead.
//!
//! It parses remote-controlled input, so depth and token length are bounded and
//! every failure is a typed error rather than a panic or an unbounded
//! recursion.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// What the reader refuses to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonError {
    Depth { limit: usize },
    NumberTooLong { limit: usize },
    StringTooLong { limit: usize },
    Malformed { at: usize },
    Trailing { at: usize },
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Depth { limit } => {
                write!(formatter, "nesting deeper than {limit} levels")
            }
            Self::NumberTooLong { limit } => {
                write!(formatter, "a numeric literal longer than {limit} bytes")
            }
            Self::StringTooLong { limit } => {
                write!(formatter, "a string longer than {limit} bytes")
            }
            Self::Malformed { at } => {
                write!(formatter, "malformed JSON at byte {at}")
            }
            Self::Trailing { at } => {
                write!(formatter, "trailing input at byte {at}")
            }
        }
    }
}

impl Error for JsonError {}

/// The bounds the reader enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_depth: usize,
    pub max_number_bytes: usize,
    pub max_string_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_number_bytes: 128,
            max_string_bytes: 4 * 1024 * 1024,
        }
    }
}

/// A JSON value whose numbers keep the bytes the tracker sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    Array(Vec<Node>),
    /// A `BTreeMap`, so canonical output is key-sorted.
    Object(BTreeMap<String, Node>),
}

impl Node {
    /// The child at a slash-separated path, as `serde_json`'s `pointer` does.
    #[must_use]
    pub fn at(&self, pointer: &str) -> Option<&Self> {
        let mut current = self;
        for segment in pointer.split('/').filter(|part| !part.is_empty()) {
            current = match current {
                Self::Object(fields) => fields.get(segment)?,
                _ => return None,
            };
        }
        Some(current)
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    /// Compact, key-sorted output, with every number emitted as the token it
    /// arrived as.
    ///
    /// Strings are escaped by `serde_json`, so their rendering matches the
    /// committed corpus.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Number(token) => out.push_str(token),
            Self::Text(value) => out.push_str(
                &serde_json::to_string(value)
                    .unwrap_or_else(|_| String::from("\"\"")),
            ),
            Self::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write_canonical(out);
                }
                out.push(']');
            }
            Self::Object(fields) => {
                out.push('{');
                for (index, (name, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(
                        &serde_json::to_string(name)
                            .unwrap_or_else(|_| String::from("\"\"")),
                    );
                    out.push(':');
                    value.write_canonical(out);
                }
                out.push('}');
            }
        }
    }
}

/// Reads one JSON document under `limits`.
///
/// # Errors
///
/// [`JsonError`] for malformed input, trailing bytes, or a document breaching
/// one of the bounds.
pub fn parse(text: &str, limits: &Limits) -> Result<Node, JsonError> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
        limits: *limits,
    };
    reader.skip_space();
    let node = reader.value(0)?;
    reader.skip_space();
    if reader.at != reader.bytes.len() {
        return Err(JsonError::Trailing { at: reader.at });
    }
    Ok(node)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    limits: Limits,
}

impl Reader<'_> {
    const fn malformed<T>(&self) -> Result<T, JsonError> {
        Err(JsonError::Malformed { at: self.at })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), JsonError> {
        if self.peek() == Some(byte) {
            self.at += 1;
            return Ok(());
        }
        self.malformed()
    }

    fn literal(&mut self, word: &str) -> Result<(), JsonError> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            return Ok(());
        }
        self.malformed()
    }

    fn value(&mut self, depth: usize) -> Result<Node, JsonError> {
        if depth > self.limits.max_depth {
            return Err(JsonError::Depth {
                limit: self.limits.max_depth,
            });
        }
        match self.peek() {
            Some(b'n') => {
                self.literal("null")?;
                Ok(Node::Null)
            }
            Some(b't') => {
                self.literal("true")?;
                Ok(Node::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(Node::Bool(false))
            }
            Some(b'"') => Ok(Node::Text(self.text()?)),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => {
                Ok(Node::Number(self.number()?))
            }
            _ => self.malformed(),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Node, JsonError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Node::Array(items));
        }
        loop {
            self.skip_space();
            items.push(self.value(depth + 1)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Node::Array(items));
                }
                _ => return self.malformed(),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Node, JsonError> {
        self.expect(b'{')?;
        let mut fields = BTreeMap::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Node::Object(fields));
        }
        loop {
            self.skip_space();
            let name = self.text()?;
            self.skip_space();
            self.expect(b':')?;
            self.skip_space();
            let value = self.value(depth + 1)?;
            fields.insert(name, value);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Node::Object(fields));
                }
                _ => return self.malformed(),
            }
        }
    }

    /// A number's source bytes, kept verbatim after a shape check.
    fn number(&mut self) -> Result<String, JsonError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        let digits = self.digits();
        if digits == 0 {
            return self.malformed();
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return self.malformed();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return self.malformed();
            }
        }
        let token = &self.bytes[start..self.at];
        if token.len() > self.limits.max_number_bytes {
            return Err(JsonError::NumberTooLong {
                limit: self.limits.max_number_bytes,
            });
        }
        String::from_utf8(token.to_vec()).or_else(|_| self.malformed())
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        self.at - start
    }

    /// A string, decoded through `serde_json` so escaping and surrogate
    /// handling match the rest of the workspace exactly.
    fn text(&mut self) -> Result<String, JsonError> {
        let start = self.at;
        self.expect(b'"')?;
        loop {
            match self.peek() {
                Some(b'\\') => self.at += 2,
                Some(b'"') => {
                    self.at += 1;
                    break;
                }
                Some(_) => self.at += 1,
                None => return self.malformed(),
            }
            if self.at.saturating_sub(start) > self.limits.max_string_bytes {
                return Err(JsonError::StringTooLong {
                    limit: self.limits.max_string_bytes,
                });
            }
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| JsonError::Malformed { at: start })?;
        serde_json::from_str::<String>(raw)
            .map_err(|_| JsonError::Malformed { at: start })
    }
}
