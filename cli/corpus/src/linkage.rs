//! Body-section typed-linkage extraction.
//!
//! Walks the five linkage-bearing H2 sections of a meta document and emits one
//! record per candidate reference: the inferred key, the typed target, a stable
//! anchor, and a confidence band.
//!
//! Keyword boundaries are matched against an explicit character set rather than
//! a word-boundary class, so hyphenated and underscored compounds (`code-block`,
//! `code_block`) never read as the bare keyword.

/// How confidently a reference was classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Resolved,
    Ambiguous,
}

impl Band {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Ambiguous => "ambiguous",
        }
    }
}

/// One extracted reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkageRecord {
    pub source_type: String,
    pub key: String,
    pub target_ref: String,
    pub anchor: String,
    pub band: Band,
}

/// The H2 sections that carry linkage.
pub const SECTIONS: [&str; 5] = [
    "## References",
    "## Dependencies",
    "## Historical Context",
    "## Related Research",
    "## Source References",
];

/// The valid `(source_type, key, target_type)` pairings.
const TYPE_PAIRS: [(&str, &str, &str); 14] = [
    ("work-item", "parent", "work-item"),
    ("plan", "parent", "work-item"),
    ("adr", "supersedes", "adr"),
    ("work-item", "blocks", "work-item"),
    ("plan-review", "target", "plan"),
    ("work-item-review", "target", "work-item"),
    ("plan-validation", "target", "plan"),
    ("plan", "derived_from", "codebase-research"),
    ("plan", "derived_from", "issue-research"),
    ("work-item", "derived_from", "note"),
    ("work-item", "derived_from", "work-item"),
    ("adr", "relates_to", "adr"),
    ("design-gap", "relates_to", "design-inventory"),
    ("work-item", "source", "note"),
];

const PATH_TYPES: [(&str, &str); 12] = [
    ("/reviews/plans/", "plan-review"),
    ("/reviews/work/", "work-item-review"),
    ("/reviews/prs/", "pr-review"),
    ("/work/", "work-item"),
    ("/plans/", "plan"),
    ("/decisions/", "adr"),
    ("/research/codebase/", "codebase-research"),
    ("/research/issues/", "issue-research"),
    ("/research/design-gaps/", "design-gap"),
    ("/research/design-inventories/", "design-inventory"),
    ("/validations/", "plan-validation"),
    ("/notes/", "note"),
];

/// True when a token is a documentation placeholder rather than a real link.
#[must_use]
pub fn is_template_path(token: &str) -> bool {
    if token.contains("NNNN") || token.contains("YYYY-MM-DD") {
        return true;
    }
    token
        .find('{')
        .is_some_and(|open| token[open..].contains('}'))
}

const fn is_before_boundary(character: char) -> bool {
    matches!(character, ' ' | '\t' | '(' | '[' | '"' | '\'')
}

const fn is_after_boundary(character: char) -> bool {
    matches!(character, ' ' | '\t' | ':' | ',' | '.' | ')' | '"' | '\'')
}

fn contains_word(line: &str, word: &str, fold_case: bool) -> bool {
    let haystack: Vec<char> = line.chars().collect();
    let needle: Vec<char> = word.chars().collect();
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    for start in 0..=(haystack.len() - needle.len()) {
        let hit = (0..needle.len()).all(|offset| {
            let found = haystack[start + offset];
            let wanted = needle[offset];
            if fold_case {
                found.eq_ignore_ascii_case(&wanted)
            } else {
                found == wanted
            }
        });
        if !hit {
            continue;
        }
        let end = start + needle.len();
        let before = start == 0 || is_before_boundary(haystack[start - 1]);
        let after = end == haystack.len() || is_after_boundary(haystack[end]);
        if before && after {
            return true;
        }
    }
    false
}

/// True when the line carries a standalone `block`/`blocks` keyword.
#[must_use]
pub fn has_blocks_keyword(line: &str) -> bool {
    ["Blocks", "blocks", "Block", "block"]
        .iter()
        .any(|word| contains_word(line, word, false))
}

/// True when the line carries a standalone `blocked by`/`blocked-by` label.
#[must_use]
pub fn has_blocked_by_keyword(line: &str) -> bool {
    ["blocked by", "blocked-by"]
        .iter()
        .any(|word| contains_word(line, word, true))
}

/// True when the line carries a standalone `sibling` keyword.
#[must_use]
pub fn has_sibling_keyword(line: &str) -> bool {
    ["Sibling", "sibling"]
        .iter()
        .any(|word| contains_word(line, word, false))
}

/// True when the line carries a standalone `supersede`/`supersedes` keyword.
#[must_use]
pub fn has_supersedes_keyword(line: &str) -> bool {
    ["supersedes", "supersede"]
        .iter()
        .any(|word| contains_word(line, word, true))
}

fn after_list_marker(line: &str) -> &str {
    let rest = line.trim_start();
    let rest = rest
        .strip_prefix('-')
        .or_else(|| rest.strip_prefix('*'))
        .unwrap_or(rest);
    rest.trim_start()
}

/// True when the line leads with a `Source:` label.
#[must_use]
pub fn has_source_label(line: &str) -> bool {
    let rest = after_list_marker(line);
    rest.starts_with("Source:") || rest.starts_with("source:")
}

fn has_dependency_label(line: &str) -> bool {
    const LABELS: [&str; 9] = [
        "blocked by",
        "blocked-by",
        "depends on",
        "depend on",
        "blocks",
        "block",
        "related",
        "sibling",
        "parent",
    ];
    let rest = after_list_marker(line);
    LABELS.iter().any(|label| {
        rest.get(..label.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(label))
            && rest[label.len()..].starts_with(':')
    })
}

/// Maps a meta path to its document type.
#[must_use]
pub fn type_from_path(path: &str) -> Option<&'static str> {
    PATH_TYPES
        .iter()
        .find(|(segment, _)| path.contains(segment))
        .map(|(_, kind)| *kind)
}

/// Resolves a meta path to its `(type, id)` target, or `None` when the path is
/// outside the mapped directories.
#[must_use]
pub fn resolve_path_target(path: &str) -> Option<(&'static str, String)> {
    let kind = type_from_path(path)?;
    let file = path.rsplit('/').next()?;
    let stem = file.strip_suffix(".md").unwrap_or(file);

    let id = match kind {
        "work-item" => stem.chars().take_while(char::is_ascii_digit).collect(),
        "adr" => stem.strip_prefix("ADR-").map_or_else(String::new, |rest| {
            let digits: String =
                rest.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                String::new()
            } else {
                format!("ADR-{digits}")
            }
        }),
        "design-inventory" => path
            .rsplit_once('/')
            .and_then(|(dir, _)| dir.rsplit('/').next())
            .unwrap_or_default()
            .to_owned(),
        _ => stem.to_owned(),
    };

    if id.is_empty() {
        None
    } else {
        Some((kind, id))
    }
}

fn canonical_key(key: &str) -> &str {
    match key {
        "blocked_by" => "blocks",
        "superseded_by" => "supersedes",
        other => other,
    }
}

fn pair_target_types(source: &str, key: &str) -> Vec<&'static str> {
    let key = canonical_key(key);
    TYPE_PAIRS
        .iter()
        .filter(|(pair_source, pair_key, _)| {
            *pair_source == source && *pair_key == key
        })
        .map(|(_, _, target)| *target)
        .collect()
}

fn pair_in_table(source: &str, key: &str, target: &str) -> bool {
    let key = canonical_key(key);
    TYPE_PAIRS
        .iter()
        .any(|(pair_source, pair_key, pair_target)| {
            *pair_source == source && *pair_key == key && *pair_target == target
        })
}

/// Infers the linkage key for a reference, and whether an explicit prose hint
/// fired (high confidence) rather than a section default.
#[must_use]
pub fn infer_key(
    section: &str,
    line: &str,
    target_type: &str,
) -> (String, bool) {
    if has_sibling_keyword(line) {
        return ("relates_to".to_owned(), true);
    }
    if has_supersedes_keyword(line) {
        return ("supersedes".to_owned(), true);
    }
    if has_blocked_by_keyword(line) {
        return ("blocked_by".to_owned(), true);
    }
    if has_blocks_keyword(line) {
        return ("blocks".to_owned(), true);
    }
    if has_source_label(line) {
        let key = match target_type {
            "work-item" => "parent",
            "codebase-research" | "issue-research" => "derived_from",
            _ => "source",
        };
        return (key.to_owned(), true);
    }
    let key = match section {
        "## Related Research" => "derived_from",
        "## Source References" => "source",
        _ => "relates_to",
    };
    (key.to_owned(), false)
}

/// Classifies a reference's band, filling in a bare target's type when the
/// `(source, key)` pairing is single-valued.
#[must_use]
pub fn classify_band(
    source: &str,
    key: &str,
    target_type: &str,
    explicit: bool,
) -> (Band, &'static str) {
    if target_type.is_empty() {
        let candidates = pair_target_types(source, key);
        if explicit && candidates.len() == 1 {
            return (Band::Resolved, candidates[0]);
        }
        return (Band::Ambiguous, "");
    }
    if explicit
        && (key == "relates_to"
            || key == "source"
            || pair_in_table(source, key, target_type))
    {
        return (Band::Resolved, "");
    }
    (Band::Ambiguous, "")
}

fn extract_meta_paths(line: &str) -> Vec<String> {
    let allowed = |character: char| {
        character.is_ascii_alphanumeric()
            || matches!(character, '/' | '_' | '.' | '-')
    };
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find("meta/") {
        let start = cursor + offset;
        let mut end = start;
        for (index, character) in line[start..].char_indices() {
            if allowed(character) {
                end = start + index + character.len_utf8();
            } else {
                break;
            }
        }
        let run = &line[start..end];
        if let Some(suffix) = run.rfind(".md") {
            let token_end = start + suffix + ".md".len();
            tokens.push(line[start..token_end].to_owned());
            cursor = token_end;
        } else {
            cursor = start + "meta/".len();
        }
    }
    tokens
}

fn extract_adr_ids(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find("ADR-") {
        let after = cursor + offset + "ADR-".len();
        let digits: String = line[after..]
            .chars()
            .take_while(char::is_ascii_digit)
            .take(4)
            .collect();
        if digits.len() >= 3 {
            cursor = after + digits.len();
            tokens.push(format!("ADR-{digits}"));
        } else {
            cursor = after;
        }
    }
    tokens
}

fn extract_pr_refs(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find("pr:") {
        let after = cursor + offset + "pr:".len();
        let digits: String = line[after..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if digits.is_empty() {
            cursor = after;
        } else {
            cursor = after + digits.len();
            tokens.push(format!("pr:{digits}"));
        }
    }
    tokens
}

fn extract_bare_ids(line: &str) -> Vec<String> {
    let characters: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index + 4 <= characters.len() {
        if characters[index..index + 4]
            .iter()
            .all(char::is_ascii_digit)
        {
            tokens.push(characters[index..index + 4].iter().collect());
            index += 4;
        } else {
            index += 1;
        }
    }
    tokens
}

fn extract_tokens(section: &str, line: &str) -> Vec<String> {
    let mut tokens = extract_meta_paths(line);
    tokens.extend(extract_adr_ids(line));
    tokens.extend(extract_pr_refs(line));
    if section == "## Dependencies" && has_dependency_label(line) {
        tokens.extend(extract_bare_ids(line));
    }
    tokens
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn classify_token(token: &str) -> (&'static str, String) {
    if token.starts_with("meta/") && token.ends_with(".md") {
        return resolve_path_target(token)
            .map_or(("", String::new()), |(kind, id)| (kind, id));
    }
    if token.starts_with("ADR-") {
        return ("adr", token.to_owned());
    }
    if let Some(number) = token.strip_prefix("pr:") {
        return ("pr", number.to_owned());
    }
    ("", token.to_owned())
}

fn section_slug(section: &str) -> String {
    section
        .strip_prefix("## ")
        .unwrap_or(section)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Extracts every linkage record from a document body.
#[must_use]
pub fn parse_document(source_type: &str, content: &str) -> Vec<LinkageRecord> {
    let mut records: Vec<LinkageRecord> = Vec::new();
    let mut section: Option<&'static str> = None;
    let mut seq = 0usize;

    for line in content.lines() {
        if line.starts_with("## ") {
            section = SECTIONS.iter().copied().find(|known| *known == line);
            continue;
        }
        let Some(section) = section else {
            continue;
        };

        let mut seen: Vec<(String, String)> = Vec::new();
        for token in extract_tokens(section, line) {
            if is_template_path(&token) {
                continue;
            }
            let (mut target_type, target_id) = classify_token(&token);
            let (key, explicit) = infer_key(section, line, target_type);
            let (band, filled) =
                classify_band(source_type, &key, target_type, explicit);
            if !filled.is_empty() {
                target_type = filled;
            }

            let target_ref = if !target_type.is_empty() && !target_id.is_empty()
            {
                format!("{target_type}:{target_id}")
            } else if target_id.is_empty() {
                token.clone()
            } else {
                target_id.clone()
            };

            if seen.iter().any(|(known_key, known_ref)| {
                *known_key == key && *known_ref == target_ref
            }) {
                continue;
            }
            seen.push((key.clone(), target_ref.clone()));

            records.push(LinkageRecord {
                source_type: source_type.to_owned(),
                key,
                target_ref,
                anchor: format!("body:{}#{seq}", section_slug(section)),
                band,
            });
            seq += 1;
        }
    }

    records
}

#[cfg(test)]
mod tests {
    use super::{
        has_blocks_keyword, has_sibling_keyword, infer_key, is_template_path,
        parse_document, Band, LinkageRecord,
    };

    fn find<'a>(
        records: &'a [LinkageRecord],
        target: &str,
    ) -> Option<&'a LinkageRecord> {
        records.iter().find(|record| record.target_ref == target)
    }

    #[test]
    fn template_placeholders_are_not_links() {
        assert!(is_template_path("meta/decisions/ADR-NNNN.md"));
        assert!(is_template_path("meta/notes/YYYY-MM-DD-topic.md"));
        assert!(is_template_path("meta/work/{number}-description.md"));
        assert!(!is_template_path("meta/work/0030-foo.md"));
    }

    #[test]
    fn a_template_reference_yields_no_record_but_a_real_one_does() {
        let content = "## References\n\
             - The template is `meta/decisions/ADR-NNNN-description.md`\n\
             - A real ref `meta/work/0030-real.md`\n";
        let records = parse_document("plan", content);
        assert!(records.iter().all(|r| !r.target_ref.contains("NNNN")));
        assert!(find(&records, "work-item:0030").is_some());
    }

    #[test]
    fn the_blocks_keyword_respects_hyphen_and_underscore_boundaries() {
        assert!(has_blocks_keyword("Blocks: 0034"));
        assert!(has_blocks_keyword("this blocks the other"));
        assert!(!has_blocks_keyword("see the code-block example"));
        assert!(!has_blocks_keyword("a code_block here"));
    }

    #[test]
    fn code_block_prose_yields_no_linkage() {
        let content =
            "## Dependencies\n- The code-block rendering is unrelated\n";
        assert!(parse_document("work-item", content).is_empty());
    }

    #[test]
    fn a_sibling_reference_is_keyed_relates_to() {
        assert!(has_sibling_keyword("Sibling: meta/plans/x.md"));
        assert!(!has_sibling_keyword("siblings-list here"));

        let content = "## References\n\
             - Sibling component plans: `meta/plans/2026-01-03-0003-other.md`\n";
        let records = parse_document("plan", content);
        let record = find(&records, "plan:2026-01-03-0003-other");
        assert_eq!(record.map(|r| r.key.as_str()), Some("relates_to"));
    }

    #[test]
    fn bands_classify_across_the_five_headers() {
        let blocks =
            parse_document("work-item", "## Dependencies\n- Blocks: 0061\n");
        assert_eq!(
            find(&blocks, "work-item:0061").map(|r| r.band),
            Some(Band::Resolved)
        );

        let source = parse_document(
            "plan",
            "## References\n- Source: `meta/work/0063-owning.md`\n",
        );
        assert_eq!(
            find(&source, "work-item:0063").map(|r| r.band),
            Some(Band::Resolved)
        );

        let unhinted = parse_document(
            "plan",
            "## References\n- `meta/plans/2026-02-03-0065-bare.md`\n",
        );
        assert_eq!(
            find(&unhinted, "plan:2026-02-03-0065-bare").map(|r| r.band),
            Some(Band::Ambiguous)
        );

        let research = parse_document(
            "plan",
            "## Related Research\n- `meta/research/codebase/2026-02-04-rr.md`\n",
        );
        assert_eq!(
            find(&research, "codebase-research:2026-02-04-rr").map(|r| r.band),
            Some(Band::Ambiguous)
        );

        let supersedes = parse_document(
            "adr",
            "## Historical Context\n\
             - Supersedes `meta/decisions/ADR-0026-old.md`\n",
        );
        assert_eq!(
            find(&supersedes, "adr:ADR-0026").map(|r| r.band),
            Some(Band::Resolved)
        );
    }

    #[test]
    fn a_plan_target_keeps_its_full_stem() {
        let content = "## References\n\
             - Sibling: `meta/plans/2026-05-13-0055-sidebar-activity-feed.md`\n";
        let records = parse_document("plan", content);
        assert!(find(&records, "plan:2026-05-13-0055-sidebar-activity-feed")
            .is_some());
    }

    #[test]
    fn a_source_label_disambiguates_by_target_type() {
        let line = "- Source: `meta/work/0042-x.md`";
        assert_eq!(infer_key("## References", line, "work-item").0, "parent");

        let line = "- Source: `meta/research/codebase/x.md`";
        assert_eq!(
            infer_key("## References", line, "codebase-research").0,
            "derived_from"
        );

        let line = "- Source: `meta/research/issues/x.md`";
        assert_eq!(
            infer_key("## References", line, "issue-research").0,
            "derived_from"
        );

        let line = "- Source: https://example.com/spec";
        assert_eq!(infer_key("## References", line, "").0, "source");
    }

    #[test]
    fn a_pr_reference_is_tolerated() {
        let records =
            parse_document("pr-review", "## References\n- Reviews `pr:42`\n");
        assert!(find(&records, "pr:42").is_some());
    }

    #[test]
    fn bare_and_unhinted_references_are_ambiguous() {
        let bare =
            parse_document("work-item", "## Dependencies\n- Related: 0030\n");
        assert_eq!(find(&bare, "0030").map(|r| r.band), Some(Band::Ambiguous));

        let note = parse_document(
            "work-item",
            "## References\n- `meta/notes/2026-01-01-some-note.md`\n",
        );
        assert_eq!(
            find(&note, "note:2026-01-01-some-note").map(|r| r.band),
            Some(Band::Ambiguous)
        );
    }
}
