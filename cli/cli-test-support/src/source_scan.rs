//! A lexical reading of Rust source for boundary tests: the production code
//! a file compiles to outside tests, and the places it names a built-in doc
//! type.

use std::path::{Path, PathBuf};

const TEST_GATES: [&str; 2] = ["#[cfg(test)]", "#[cfg(all(test"];

/// `source` with its comments, literals and test-gated items blanked out,
/// keeping its line breaks so line numbers still point into the file.
#[must_use]
pub fn production_code(source: &str) -> String {
    let code = code_only(source);
    let bytes = code.as_bytes();
    let mut production = String::with_capacity(code.len());
    let mut cursor = 0;
    while let Some(offset) = TEST_GATES
        .iter()
        .filter_map(|gate| code[cursor..].find(gate))
        .min()
    {
        let gate = cursor + offset;
        let end = end_of_gated_item(bytes, gate);
        production.push_str(&code[cursor..gate]);
        production.extend(code[gate..end].chars().filter(|&c| c == '\n'));
        cursor = end;
    }
    production.push_str(&code[cursor..]);
    production
}

fn code_only(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut code: Vec<u8> = bytes
        .iter()
        .map(|&byte| if byte == b'\n' { b'\n' } else { b' ' })
        .collect();
    let mut scanner = Scanner::new(bytes);
    while let Some(position) = scanner.next_code_position() {
        code[position] = bytes[position];
    }
    String::from_utf8_lossy(&code).into_owned()
}

/// The lines of `source`'s production code that name a built-in doc type,
/// each as `<line number>: <line>`.
///
/// A line names one when it reads a `DocTypeKey` constant, under its own
/// name or an alias, or imports from the corpus's built-in declarations.
#[must_use]
pub fn built_in_doc_type_references(source: &str) -> Vec<String> {
    let production = production_code(source);
    let mut key_names = vec!["DocTypeKey".to_owned()];
    key_names.extend(aliases_of_doc_type_key(&production));
    production
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            line.contains("doc_type::built_in")
                || key_names.iter().any(|name| names_a_constant(line, name))
        })
        .map(|(index, line)| format!("{}: {}", index + 1, line.trim()))
        .collect()
}

/// Every `.rs` file under each `src` directory beneath `root`, skipping
/// build output.
#[must_use]
pub fn rust_sources_under(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    collect_sources(root, false, &mut sources);
    sources.sort();
    sources
}

fn collect_sources(directory: &Path, in_src: bool, sources: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "target"
                || name == "node_modules"
                || name.starts_with('.')
            {
                continue;
            }
            collect_sources(&path, in_src || name == "src", sources);
        } else if in_src && name.ends_with(".rs") {
            sources.push(path);
        }
    }
}

fn aliases_of_doc_type_key(production: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let mut rest = production;
    while let Some(index) = rest.find("DocTypeKey as ") {
        let after = &rest[index + "DocTypeKey as ".len()..];
        let alias: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !alias.is_empty() {
            aliases.push(alias);
        }
        rest = after;
    }
    for line in production.lines() {
        let Some(declaration) =
            line.trim().strip_prefix("type ").or_else(|| {
                line.trim()
                    .strip_prefix("pub type ")
                    .or_else(|| line.trim().strip_prefix("pub(crate) type "))
            })
        else {
            continue;
        };
        if let Some((alias, aliased)) = declaration.split_once('=') {
            if aliased.trim().trim_end_matches(';').ends_with("DocTypeKey") {
                aliases.push(alias.trim().to_owned());
            }
        }
    }
    aliases
}

fn names_a_constant(line: &str, key_name: &str) -> bool {
    let qualified = format!("{key_name}::");
    line.match_indices(&qualified).any(|(index, _)| {
        let preceded_by_identifier = line[..index]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        !preceded_by_identifier
            && line[index + qualified.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
    })
}

/// The end of the item a test gate at `gate` applies to: past its closing
/// brace, or past the `;` or `,` that ends it.
fn end_of_gated_item(code: &[u8], gate: usize) -> usize {
    let mut depth: usize = 0;
    let mut in_attribute = true;
    for (position, byte) in code.iter().enumerate().skip(gate) {
        match byte {
            b'[' | b'(' | b'{' => depth += 1,
            b']' | b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && in_attribute && *byte == b']' {
                    in_attribute = next_is_attribute(code, position + 1);
                }
            }
            b'}' => {
                if depth == 0 {
                    return position;
                }
                depth -= 1;
                if depth == 0 && !in_attribute {
                    return position + 1;
                }
            }
            b';' | b',' if depth == 0 && !in_attribute => {
                return position + 1;
            }
            _ => {}
        }
    }
    code.len()
}

fn next_is_attribute(code: &[u8], from: usize) -> bool {
    code[from..]
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| *byte == b'#')
}

/// Walks source bytes, yielding the positions of code outside comments and
/// string or character literals.
struct Scanner<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Scanner<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn next_code_position(&mut self) -> Option<usize> {
        while self.position < self.bytes.len() {
            let start = self.position;
            match (self.bytes[start], self.bytes.get(start + 1)) {
                (b'/', Some(b'/')) => self.skip_past(b"\n"),
                (b'/', Some(b'*')) => self.skip_past(b"*/"),
                (b'"', _) => self.skip_string(start + 1),
                (b'r', Some(b'"' | b'#')) if self.starts_raw_string(start) => {
                    self.skip_raw_string(start + 1);
                }
                (b'\'', _) if self.starts_char_literal(start) => {
                    self.skip_char_literal(start + 1);
                }
                _ => {
                    self.position += 1;
                    return Some(start);
                }
            }
        }
        None
    }

    fn skip_past(&mut self, terminator: &[u8]) {
        self.position = self.bytes[self.position + 2..]
            .windows(terminator.len())
            .position(|window| window == terminator)
            .map_or(self.bytes.len(), |offset| {
                self.position + 2 + offset + terminator.len()
            });
    }

    fn skip_string(&mut self, mut position: usize) {
        while position < self.bytes.len() {
            match self.bytes[position] {
                b'\\' => position += 2,
                b'"' => {
                    self.position = position + 1;
                    return;
                }
                _ => position += 1,
            }
        }
        self.position = self.bytes.len();
    }

    fn starts_raw_string(&self, start: usize) -> bool {
        let preceded_by_identifier = start > 0
            && (self.bytes[start - 1].is_ascii_alphanumeric()
                || self.bytes[start - 1] == b'_');
        let hashes = self.bytes[start + 1..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        !preceded_by_identifier
            && self.bytes.get(start + 1 + hashes) == Some(&b'"')
    }

    fn skip_raw_string(&mut self, from: usize) {
        let hashes = self.bytes[from..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        let body = from + hashes + 1;
        let mut terminator = vec![b'"'];
        terminator.extend(std::iter::repeat_n(b'#', hashes));
        self.position = self.bytes[body..]
            .windows(terminator.len())
            .position(|window| window == terminator.as_slice())
            .map_or(self.bytes.len(), |offset| {
                body + offset + terminator.len()
            });
    }

    fn starts_char_literal(&self, start: usize) -> bool {
        match self.bytes.get(start + 1) {
            Some(b'\\') => true,
            Some(_) => {
                let width = utf8_width(self.bytes[start + 1]);
                self.bytes.get(start + 1 + width) == Some(&b'\'')
            }
            None => false,
        }
    }

    fn skip_char_literal(&mut self, from: usize) {
        self.position = self.bytes[from + 1..]
            .iter()
            .position(|byte| *byte == b'\'')
            .map_or(self.bytes.len(), |offset| from + 1 + offset + 1);
    }
}

const fn utf8_width(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}
