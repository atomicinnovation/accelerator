//! Body-section typed-linkage extraction.
//!
//! Walks the five linkage-bearing H2 sections of a meta document and emits one
//! record per candidate reference: the inferred key, the typed target, a stable
//! anchor, and a confidence band.
//!
//! Keyword boundaries are matched against an explicit character set rather than
//! a word-boundary class, so hyphenated and underscored compounds (`code-block`,
//! `code_block`) never read as the bare keyword.
//!
//! The directory-to-type fact is not re-encoded here: a path resolves through
//! the same injected doc-type table and the same matcher the rest of the domain
//! uses, and its vocabulary name comes from `DocTypeKey`.

use std::path::Path;
use std::path::PathBuf;

use crate::doc_type::DocTypeKey;

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
///
/// Mirrors `scripts/linkage-type-pairs.tsv`, which the bash parser reads at
/// runtime. A `corpus-adapters` suite asserts the two agree row for row.
pub const TYPE_PAIRS: [(&str, &str, &str); 16] = [
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
    ("work-item", "relates_to", "pr-description"),
    ("plan", "relates_to", "pr-description"),
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

/// Maps a meta path to its typed-linkage vocabulary name, resolving through the
/// injected doc-type table.
#[must_use]
pub fn type_from_path(
    path: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<&'static str> {
    crate::doc_type::infer(Path::new(path), table)?.linkage_type_name()
}

/// Resolves a meta path to its `(type, id)` target, or `None` when the path is
/// outside every configured doc-type directory.
#[must_use]
pub fn resolve_path_target(
    path: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<(&'static str, String)> {
    let kind = crate::doc_type::infer(Path::new(path), table)?;
    let name = kind.linkage_type_name()?;
    let file = path.rsplit('/').next()?;
    let stem = file.strip_suffix(".md").unwrap_or(file);

    let id = match kind {
        DocTypeKey::WorkItems => {
            stem.chars().take_while(char::is_ascii_digit).collect()
        }
        DocTypeKey::Decisions => {
            stem.strip_prefix("ADR-").map_or_else(String::new, |rest| {
                let digits: String =
                    rest.chars().take_while(char::is_ascii_digit).collect();
                if digits.is_empty() {
                    String::new()
                } else {
                    format!("ADR-{digits}")
                }
            })
        }
        DocTypeKey::DesignInventories => path
            .rsplit_once('/')
            .and_then(|(dir, _)| dir.rsplit('/').next())
            .unwrap_or_default()
            .to_owned(),
        DocTypeKey::PrDescriptions => {
            pr_number(stem).unwrap_or_else(|| stem.to_owned())
        }
        _ => stem.to_owned(),
    };

    if id.is_empty() {
        None
    } else {
        Some((name, id))
    }
}

/// The PR number a stem carries.
///
/// The digits of a genuine `pr-`/`PR-` *segment* — at the start, or preceded by a
/// hyphen, so a `pr` inside a word like `expr-3` does not match — else a leading
/// number on a stem that is not date-prefixed. A date-prefixed stem with no `pr`
/// token has no derivable PR number: its leading digits are a year.
#[must_use]
pub fn pr_number(stem: &str) -> Option<String> {
    let bytes = stem.as_bytes();
    for index in 0..bytes.len().saturating_sub(1) {
        let at_segment_start = index == 0 || bytes[index - 1] == b'-';
        if !at_segment_start
            || !bytes[index].eq_ignore_ascii_case(&b'p')
            || !bytes[index + 1].eq_ignore_ascii_case(&b'r')
        {
            continue;
        }

        let mut rest = index + 2;
        if bytes.get(rest) == Some(&b'-') {
            rest += 1;
        }
        let digits: String = stem
            .get(rest..)?
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }

    if is_date_prefixed(stem) {
        return None;
    }
    let digits: String =
        stem.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn is_date_prefixed(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
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

/// The distinct leading segments of the configured doc-type directories — the
/// roots a path token must sit under to be a candidate reference.
///
/// Scanning by root rather than by full directory keeps an out-of-scope subtree
/// (`meta/docs/…`) a *candidate*, exactly as a literal `meta/` scan did: it is
/// extracted, fails to infer a type, and is carried through as a raw path.
#[must_use]
pub fn path_roots(table: &[(DocTypeKey, PathBuf)]) -> Vec<String> {
    let mut roots: Vec<String> = Vec::new();
    for (_, dir) in table {
        let Some(root) = dir
            .to_str()
            .and_then(|dir| dir.split('/').next())
            .filter(|root| !root.is_empty())
        else {
            continue;
        };
        if !roots.iter().any(|seen| seen == root) {
            roots.push(root.to_owned());
        }
    }
    roots
}

/// Extracts every path token under a configured root. The root set comes from
/// the injected table, so a re-pathed corpus is scanned where it actually lives
/// rather than under a hardcoded `meta/`.
fn extract_doc_paths(line: &str, roots: &[String]) -> Vec<String> {
    let allowed = |character: char| {
        character.is_ascii_alphanumeric()
            || matches!(character, '/' | '_' | '.' | '-')
    };

    let mut tokens: Vec<(usize, String)> = Vec::new();
    for root in roots {
        let prefix = format!("{root}/");
        let mut cursor = 0;
        while let Some(offset) = line[cursor..].find(&prefix) {
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
                tokens.push((start, line[start..token_end].to_owned()));
                cursor = token_end;
            } else {
                cursor = start + prefix.len();
            }
        }
    }

    // Several roots scan the same line independently, so restore source order —
    // the anchor sequence numbers are positional.
    tokens.sort_by_key(|(start, _)| *start);
    tokens.dedup_by(|a, b| a.1 == b.1 && a.0 == b.0);
    tokens.into_iter().map(|(_, token)| token).collect()
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

fn extract_tokens(section: &str, line: &str, roots: &[String]) -> Vec<String> {
    let mut tokens = extract_doc_paths(line, roots);
    tokens.extend(extract_adr_ids(line));
    tokens.extend(extract_pr_refs(line));
    if section == "## Dependencies" && has_dependency_label(line) {
        tokens.extend(extract_bare_ids(line));
    }
    tokens
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn classify_token(
    token: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> (&'static str, String) {
    // The extractor only yields path tokens ending `.md`; ADR ids, `pr:` refs and
    // bare ids never do. So the suffix identifies a path without pinning it to a
    // hardcoded root.
    if token.ends_with(".md") {
        return resolve_path_target(token, table)
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

/// Extracts every linkage record from a document body, resolving path targets
/// through the injected doc-type table.
#[must_use]
pub fn parse_document(
    source_type: &str,
    content: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Vec<LinkageRecord> {
    let mut records: Vec<LinkageRecord> = Vec::new();
    let mut section: Option<&'static str> = None;
    let mut seq = 0usize;
    let roots = path_roots(table);

    for line in content.lines() {
        if line.starts_with("## ") {
            section = SECTIONS.iter().copied().find(|known| *known == line);
            continue;
        }
        let Some(section) = section else {
            continue;
        };

        let mut seen: Vec<(String, String)> = Vec::new();
        for token in extract_tokens(section, line, &roots) {
            if is_template_path(&token) {
                continue;
            }
            let (mut target_type, target_id) = classify_token(&token, table);
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
    use std::path::PathBuf;

    use super::{
        has_blocks_keyword, has_sibling_keyword, infer_key, is_template_path,
        Band, LinkageRecord,
    };
    use crate::doc_type::DocTypeKey;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        [
            (DocTypeKey::WorkItems, "meta/work"),
            (DocTypeKey::Plans, "meta/plans"),
            (DocTypeKey::Validations, "meta/validations"),
            (DocTypeKey::PrDescriptions, "meta/prs"),
            (DocTypeKey::Decisions, "meta/decisions"),
            (DocTypeKey::Research, "meta/research/codebase"),
            (DocTypeKey::RootCauseAnalyses, "meta/research/issues"),
            (
                DocTypeKey::DesignInventories,
                "meta/research/design-inventories",
            ),
            (DocTypeKey::DesignGaps, "meta/research/design-gaps"),
            (DocTypeKey::PlanReviews, "meta/reviews/plans"),
            (DocTypeKey::WorkItemReviews, "meta/reviews/work"),
            (DocTypeKey::PrReviews, "meta/reviews/prs"),
            (DocTypeKey::Notes, "meta/notes"),
        ]
        .into_iter()
        .map(|(kind, dir)| (kind, PathBuf::from(dir)))
        .collect()
    }

    fn parse_document(source: &str, content: &str) -> Vec<LinkageRecord> {
        super::parse_document(source, content, &table())
    }

    #[test]
    fn a_re_pathed_corpus_is_scanned_where_it_actually_lives() {
        // The scan roots come from the table, so a corpus that is not under
        // `meta/` still has its references extracted. A hardcoded `meta/` prefix
        // would find nothing here.
        let table = vec![
            (DocTypeKey::WorkItems, PathBuf::from("docs/tickets")),
            (DocTypeKey::Plans, PathBuf::from("docs/plans")),
        ];
        let content = "## References\n- A sibling `docs/tickets/0002-y.md`\n";

        let records = super::parse_document("work-item", content, &table);

        assert_eq!(
            records.len(),
            1,
            "the token must be extracted: {records:?}"
        );
        assert_eq!(records[0].target_ref, "work-item:0002");
    }

    #[test]
    fn path_roots_are_the_distinct_leading_segments() {
        let table = vec![
            (DocTypeKey::WorkItems, PathBuf::from("meta/work")),
            (DocTypeKey::Plans, PathBuf::from("meta/plans")),
            (DocTypeKey::Notes, PathBuf::from("docs/notes")),
        ];
        assert_eq!(super::path_roots(&table), vec!["meta", "docs"]);
    }

    #[test]
    fn a_path_under_a_root_but_outside_every_doc_type_stays_a_raw_path() {
        // meta/docs/ is a configured root but no doc-type directory — it must
        // still be extracted and carried through unresolved, as before.
        let content = "## References\n- See `meta/docs/logging-guide.md`\n";
        let records = parse_document("work-item", content);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target_ref, "meta/docs/logging-guide.md");
        assert_eq!(records[0].band, Band::Ambiguous);
    }

    #[test]
    fn a_pr_number_comes_from_a_genuine_pr_segment() {
        assert_eq!(
            super::pr_number("pr-42-description").as_deref(),
            Some("42")
        );
        assert_eq!(
            super::pr_number("PR-42-description").as_deref(),
            Some("42")
        );
        assert_eq!(
            super::pr_number("2026-06-17-pr-430").as_deref(),
            Some("430")
        );
    }

    #[test]
    fn a_pr_token_inside_a_word_is_not_a_pr_segment() {
        // `expr-3` and `improve-2` carry a `pr` that is not a segment.
        assert_eq!(super::pr_number("expr-3-notes"), None);
        assert_eq!(super::pr_number("improve-2-thing"), None);
    }

    #[test]
    fn a_leading_number_is_the_pr_number_unless_the_stem_is_a_date() {
        assert_eq!(super::pr_number("240-description").as_deref(), Some("240"));
        assert_eq!(
            super::pr_number("2026-06-17-summary"),
            None,
            "a date-prefixed stem's leading digits are a year, not a PR number"
        );
    }

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
