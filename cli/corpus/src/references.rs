//! Rewriting every reference to a renamed artifact across a corpus file:
//! its typed links, its identifier in prose, and paths naming its file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdShape {
    /// Unlikely to occur by accident, so every whole-token occurrence is a
    /// reference.
    Distinctive,
    /// A bare number, which prose uses for many things, so only typed links
    /// are references.
    NumericOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renaming<'a> {
    pub doc_type: &'a str,
    pub old_id: &'a str,
    pub new_id: &'a str,
    pub shape: IdShape,
    /// Exact spellings of the renamed file's path and what each becomes,
    /// applied in order, so a longer spelling must precede any it contains.
    pub path_spellings: &'a [(String, String)],
}

/// The frontmatter key recording an artifact's retired identifiers, whose
/// value keeps naming the old identifier by design.
const RETIRED_IDS_KEY: &str = "aliases";

#[must_use]
pub fn rewrite_references(content: &str, renaming: &Renaming<'_>) -> String {
    let mut rewritten = String::with_capacity(content.len());
    let mut region = Region::BeforeFrontmatter;
    for (index, line) in content.split_inclusive('\n').enumerate() {
        region = region.after(index, line);
        if region.is_retired_ids() {
            rewritten.push_str(line);
        } else {
            rewritten.push_str(&rewrite_line(line, renaming));
        }
    }
    rewritten
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    BeforeFrontmatter,
    Frontmatter,
    RetiredIds,
    Body,
}

impl Region {
    fn after(self, index: usize, line: &str) -> Self {
        let text = line.trim_end_matches(['\n', '\r']);
        let is_fence = text.trim_end() == "---";
        match self {
            Self::BeforeFrontmatter if index == 0 && is_fence => {
                Self::Frontmatter
            }
            Self::BeforeFrontmatter | Self::Body => Self::Body,
            Self::Frontmatter | Self::RetiredIds if is_fence => Self::Body,
            Self::RetiredIds if text.starts_with(char::is_whitespace) => {
                Self::RetiredIds
            }
            Self::Frontmatter | Self::RetiredIds => {
                if is_key_line(text, RETIRED_IDS_KEY) {
                    Self::RetiredIds
                } else {
                    Self::Frontmatter
                }
            }
        }
    }

    const fn is_retired_ids(self) -> bool {
        matches!(self, Self::RetiredIds)
    }
}

fn is_key_line(line: &str, key: &str) -> bool {
    line.strip_prefix(key)
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
}

fn rewrite_line(line: &str, renaming: &Renaming<'_>) -> String {
    let mut rewritten = line.to_owned();
    for (from, to) in renaming.path_spellings {
        rewritten = rewritten.replace(from.as_str(), to);
    }
    match renaming.shape {
        IdShape::Distinctive => {
            replace_whole_tokens(&rewritten, renaming.old_id, renaming.new_id)
        }
        IdShape::NumericOnly => replace_whole_tokens(
            &rewritten,
            &typed_link(renaming.doc_type, renaming.old_id),
            &typed_link(renaming.doc_type, renaming.new_id),
        ),
    }
}

fn typed_link(doc_type: &str, id: &str) -> String {
    format!("{doc_type}:{id}")
}

const fn joins_a_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// Replaces each ASCII-case-insensitive occurrence of `needle` whose
/// neighbours cannot extend it into a longer token.
fn replace_whole_tokens(text: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return text.to_owned();
    }
    let folded_text = text.to_ascii_lowercase();
    let folded_needle = needle.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut rewritten = String::with_capacity(text.len());
    let mut copied_up_to = 0;
    let mut search_from = 0;
    while let Some(offset) = folded_text[search_from..].find(&folded_needle) {
        let start = search_from + offset;
        let end = start + needle.len();
        let starts_a_token = start == 0 || !joins_a_token(bytes[start - 1]);
        let ends_a_token = bytes.get(end).is_none_or(|&b| !joins_a_token(b));
        if starts_a_token && ends_a_token {
            rewritten.push_str(&text[copied_up_to..start]);
            rewritten.push_str(replacement);
            copied_up_to = end;
            search_from = end;
        } else {
            search_from = start + 1;
        }
    }
    rewritten.push_str(&text[copied_up_to..]);
    rewritten
}

#[cfg(test)]
mod tests {
    use super::rewrite_references;
    use super::IdShape;
    use super::Renaming;

    fn distinctive<'a>(old_id: &'a str, new_id: &'a str) -> Renaming<'a> {
        Renaming {
            doc_type: "work-item",
            old_id,
            new_id,
            shape: IdShape::Distinctive,
            path_spellings: &[],
        }
    }

    fn numeric<'a>(old_id: &'a str, new_id: &'a str) -> Renaming<'a> {
        Renaming {
            shape: IdShape::NumericOnly,
            ..distinctive(old_id, new_id)
        }
    }

    #[test]
    fn a_typed_link_to_the_old_id_is_rewritten() {
        let content = "---\nparent: \"work-item:draft-k7mq3x\"\n---\n";

        assert_eq!(
            rewrite_references(content, &distinctive("draft-k7mq3x", "PP-900")),
            "---\nparent: \"work-item:PP-900\"\n---\n"
        );
    }

    #[test]
    fn prose_whole_tokens_of_a_distinctive_id_are_rewritten() {
        let content = "See draft-k7mq3x, then draft-k7mq3x again.\n";

        assert_eq!(
            rewrite_references(content, &distinctive("draft-k7mq3x", "PP-900")),
            "See PP-900, then PP-900 again.\n"
        );
    }

    #[test]
    fn occurrences_adjacent_to_a_letter_digit_or_hyphen_are_left() {
        let content = "draft-k7mq3x-notes adraft-k7mq3x draft-k7mq3x9 \
                       (draft-k7mq3x) Draft-K7MQ3X\n";

        assert_eq!(
            rewrite_references(content, &distinctive("draft-k7mq3x", "PP-900")),
            "draft-k7mq3x-notes adraft-k7mq3x draft-k7mq3x9 (PP-900) PP-900\n"
        );
    }

    #[test]
    fn an_underscore_key_is_distinctive_and_rewritten_in_prose() {
        assert_eq!(
            rewrite_references(
                "Blocked on MY_PROJ-12.\n",
                &distinctive("MY_PROJ-12", "ENG-3")
            ),
            "Blocked on ENG-3.\n"
        );
    }

    #[test]
    fn a_retired_key_that_prefixes_another_leaves_the_other_untouched() {
        assert_eq!(
            rewrite_references(
                "PP-76 and PP-760\n",
                &distinctive("PP-76", "ENG-42")
            ),
            "ENG-42 and PP-760\n"
        );
    }

    #[test]
    fn a_bare_numeric_id_is_rewritten_only_in_typed_links() {
        let content = "---\nparent: \"work-item:0230\"\n---\n\
                       Item 0230 and work-item:02301.\n";

        assert_eq!(
            rewrite_references(content, &numeric("0230", "PP-760")),
            "---\nparent: \"work-item:PP-760\"\n---\n\
             Item 0230 and work-item:02301.\n"
        );
    }

    #[test]
    fn aliases_entries_are_never_rewritten() {
        let content = "---\nid: \"ENG-1\"\naliases: [\"draft-k7mq3x\"]\n\
                       parent: \"work-item:draft-k7mq3x\"\n---\n";
        let block = "---\naliases:\n  - \"draft-k7mq3x\"\n\
                     tags: [draft-k7mq3x]\n---\n";
        let renaming = distinctive("draft-k7mq3x", "PP-900");

        assert_eq!(
            rewrite_references(content, &renaming),
            "---\nid: \"ENG-1\"\naliases: [\"draft-k7mq3x\"]\n\
             parent: \"work-item:PP-900\"\n---\n"
        );
        assert_eq!(
            rewrite_references(block, &renaming),
            "---\naliases:\n  - \"draft-k7mq3x\"\ntags: [PP-900]\n---\n"
        );
    }

    #[test]
    fn an_aliases_line_in_the_body_is_prose() {
        let content = "---\nid: \"1\"\n---\naliases: draft-k7mq3x\n";

        assert_eq!(
            rewrite_references(content, &distinctive("draft-k7mq3x", "PP-900")),
            "---\nid: \"1\"\n---\naliases: PP-900\n"
        );
    }

    #[test]
    fn a_path_reference_to_the_retired_file_is_rewritten_by_its_spellings() {
        let spellings = [
            (
                "drafts/draft-k7mq3x-slug.md".to_owned(),
                "PP-900-slug.md".to_owned(),
            ),
            (
                "draft-k7mq3x-slug.md".to_owned(),
                "PP-900-slug.md".to_owned(),
            ),
        ];
        let renaming = Renaming {
            path_spellings: &spellings,
            ..distinctive("draft-k7mq3x", "PP-900")
        };

        assert_eq!(
            rewrite_references(
                "[a](meta/work/drafts/draft-k7mq3x-slug.md) b \
                 draft-k7mq3x-slug.md\n",
                &renaming
            ),
            "[a](meta/work/PP-900-slug.md) b PP-900-slug.md\n"
        );
    }
}
