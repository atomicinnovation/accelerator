//! Tag-array mutation over a whole raw frontmatter block. Port of
//! `work-item-update-tags.sh:76-163`.

use crate::show::read_field_raw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagAction {
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagMutation {
    Changed(String),
    NoChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagError {
    BlockStyleTags,
}

/// True iff `frontmatter_raw` writes `tags:` in YAML block-list style (a
/// `tags:` line with an empty value followed by an indented `- ` item).
/// Port of the block-style scan in `work-item-update-tags.sh:44-68`.
fn is_block_style_tags(frontmatter_raw: &str) -> bool {
    let mut lines = frontmatter_raw.lines();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("tags:") {
            let trimmed = rest.trim();
            if trimmed.is_empty() {
                if let Some(next_line) = lines.next() {
                    let leading_space = next_line
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace);
                    if leading_space && next_line.trim_start().starts_with('-')
                    {
                        return true;
                    }
                }
            }
            return false;
        }
    }
    false
}

fn needs_quoting(value: &str) -> bool {
    value.contains(',') || value.contains(':') || value.contains('#')
}

fn format_tag(value: &str) -> String {
    if needs_quoting(value) {
        format!("\"{value}\"")
    } else {
        value.to_owned()
    }
}

fn build_canonical(tags: &[String]) -> String {
    let formatted: Vec<String> = tags.iter().map(|t| format_tag(t)).collect();
    format!("[{}]", formatted.join(", "))
}

/// Parses a canonical tags array value (`[a, b, "c,d"]`) into its items via
/// a **naive comma split** — the same `tr ',' '\n'` bash uses
/// (`work-item-update-tags.sh:76-88`), which does not respect quoting.
///
/// Preserved quirk, not a bug: a comma-quoted existing tag (`"c,d"`) is
/// mis-split into two tokens (`c`, `d`) on the next add/remove — this is
/// the shell's actual behaviour and is reproduced as-is.
fn parse_current_tags(raw_tags: &str) -> Vec<String> {
    let stripped = raw_tags.strip_prefix('[').unwrap_or(raw_tags);
    let stripped = stripped.strip_suffix(']').unwrap_or(stripped);
    if stripped.is_empty() {
        return Vec::new();
    }
    stripped
        .split(',')
        .filter_map(|item| {
            let mut item = item
                .trim_matches(|c: char| c.is_ascii_whitespace())
                .to_owned();
            if item.starts_with(['"', '\'']) {
                item.remove(0);
            }
            if item.ends_with(['"', '\'']) {
                item.pop();
            }
            (!item.is_empty()).then_some(item)
        })
        .collect()
}

/// Mutates the `tags` array.
///
/// Takes the whole raw frontmatter text (not just the already-extracted
/// tags value) so the block-style-before-parse ordering is a structural
/// property of the function: it scans for block-style tags and rejects
/// them before ever extracting or parsing a tags value.
///
/// # Errors
///
/// [`TagError::BlockStyleTags`] when `tags:` is written in YAML block-list
/// style.
pub fn mutate_tags(
    frontmatter_raw: &str,
    action: TagAction,
    tag: &str,
) -> Result<TagMutation, TagError> {
    if is_block_style_tags(frontmatter_raw) {
        return Err(TagError::BlockStyleTags);
    }

    let raw_tags = read_field_raw(frontmatter_raw, "tags");
    let mut current = raw_tags
        .as_deref()
        .map(parse_current_tags)
        .unwrap_or_default();

    match action {
        TagAction::Add => {
            if current.iter().any(|existing| existing == tag) {
                return Ok(TagMutation::NoChange);
            }
            current.push(tag.to_owned());
            Ok(TagMutation::Changed(build_canonical(&current)))
        }
        TagAction::Remove => {
            if raw_tags.is_none() {
                return Ok(TagMutation::NoChange);
            }
            let original_len = current.len();
            current.retain(|existing| existing != tag);
            if current.len() == original_len {
                return Ok(TagMutation::NoChange);
            }
            Ok(TagMutation::Changed(build_canonical(&current)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mutate_tags, TagAction, TagError, TagMutation};

    const FRONTMATTER: &str = "tags: [a, b]\n";

    #[test]
    fn add_a_new_tag() {
        assert_eq!(
            mutate_tags(FRONTMATTER, TagAction::Add, "c"),
            Ok(TagMutation::Changed("[a, b, c]".to_owned()))
        );
    }

    #[test]
    fn add_a_duplicate_tag_is_a_no_op() {
        assert_eq!(
            mutate_tags(FRONTMATTER, TagAction::Add, "a"),
            Ok(TagMutation::NoChange)
        );
    }

    #[test]
    fn remove_a_present_tag() {
        assert_eq!(
            mutate_tags(FRONTMATTER, TagAction::Remove, "a"),
            Ok(TagMutation::Changed("[b]".to_owned()))
        );
    }

    #[test]
    fn remove_an_absent_tag_is_a_no_op() {
        assert_eq!(
            mutate_tags(FRONTMATTER, TagAction::Remove, "zzz"),
            Ok(TagMutation::NoChange)
        );
    }

    #[test]
    fn add_when_tags_field_absent() {
        assert_eq!(
            mutate_tags("status: draft\n", TagAction::Add, "x"),
            Ok(TagMutation::Changed("[x]".to_owned()))
        );
    }

    #[test]
    fn remove_when_tags_field_absent_is_a_no_op() {
        assert_eq!(
            mutate_tags("status: draft\n", TagAction::Remove, "x"),
            Ok(TagMutation::NoChange)
        );
    }

    #[test]
    fn block_style_tags_are_rejected_without_a_separate_pre_check() {
        let frontmatter = "tags:\n  - a\n  - b\n";
        assert_eq!(
            mutate_tags(frontmatter, TagAction::Add, "c"),
            Err(TagError::BlockStyleTags)
        );
    }

    #[test]
    fn comma_quoted_tag_is_naively_resplit_on_the_next_mutation() {
        let frontmatter = "tags: [a, \"c,d\"]\n";
        assert_eq!(
            mutate_tags(frontmatter, TagAction::Add, "e"),
            Ok(TagMutation::Changed("[a, c, d, e]".to_owned()))
        );
        assert_eq!(
            mutate_tags(frontmatter, TagAction::Remove, "c"),
            Ok(TagMutation::Changed("[a, d]".to_owned()))
        );
    }

    #[test]
    fn tags_needing_quoting_are_quoted_on_rebuild() {
        let frontmatter = "tags: [needs:colon, needs#hash]\n";
        assert_eq!(
            mutate_tags(frontmatter, TagAction::Add, "z"),
            Ok(TagMutation::Changed(
                "[\"needs:colon\", \"needs#hash\", z]".to_owned()
            ))
        );
    }
}
