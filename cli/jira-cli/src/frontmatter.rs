//! Minimal top-level frontmatter field extraction.
//!
//! `resolve-fields` reads `kind`, `id` and `external_id` from a work-item
//! file's frontmatter without a YAML dependency — first match wins, surrounding
//! quotes stripped — reproducing the retiring resolver's `awk` idiom.

/// The frontmatter block between the opening and closing `---`, or `None` when
/// the file carries none.
#[must_use]
pub fn block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    rest.split_once("\n---\n")
        .map(|(frontmatter, _)| frontmatter)
}

/// The first top-level `key:` value in `frontmatter`, unquoted and trimmed.
#[must_use]
pub fn field(frontmatter: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| unquote(value.trim()))
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'')
        {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}
