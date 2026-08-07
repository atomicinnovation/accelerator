//! Raw frontmatter field reads, operating on already-extracted frontmatter
//! text. Port of `work-item-read-field.sh:53-79`.

/// First-match-wins line scan for a `<field>:` prefix.
///
/// Trims both ends of the value, then strips one leading and one trailing
/// quote character (independently — a mismatched `'foo"` loses both,
/// matching the shell's own sequential-sed behaviour).
#[must_use]
pub fn read_field_raw(frontmatter: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    for line in frontmatter.lines() {
        if let Some(rest) = line.strip_prefix(prefix.as_str()) {
            let mut value = rest
                .trim_matches(|c: char| c.is_ascii_whitespace())
                .to_owned();
            if value.starts_with(['"', '\'']) {
                value.remove(0);
            }
            if value.ends_with(['"', '\'']) {
                value.pop();
            }
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::read_field_raw;

    const FRONTMATTER: &str = concat!(
        "id: \"work-item:0042\"\n",
        "status: draft\n",
        "tags: [a, b]\n",
    );

    #[test]
    fn reads_a_present_field() {
        assert_eq!(
            read_field_raw(FRONTMATTER, "status").as_deref(),
            Some("draft")
        );
    }

    #[test]
    fn strips_surrounding_quotes() {
        assert_eq!(
            read_field_raw(FRONTMATTER, "id").as_deref(),
            Some("work-item:0042")
        );
    }

    #[test]
    fn returns_array_values_verbatim() {
        assert_eq!(
            read_field_raw(FRONTMATTER, "tags").as_deref(),
            Some("[a, b]")
        );
    }

    #[test]
    fn missing_field_is_none() {
        assert_eq!(read_field_raw(FRONTMATTER, "priority"), None);
    }

    #[test]
    fn single_quoted_value() {
        assert_eq!(
            read_field_raw("title: 'single quoted'\n", "title").as_deref(),
            Some("single quoted")
        );
    }

    #[test]
    fn mismatched_quotes_are_each_stripped_independently() {
        assert_eq!(
            read_field_raw("weird: 'foo\"\n", "weird").as_deref(),
            Some("foo")
        );
    }
}
