//! Central parser for ADR-0034 typed-linkage reference values.
//!
//! **Path safety**: `TypedRef::Path` carries the raw, unvalidated
//! repo-relative path string. Callers that intend to resolve it
//! against the filesystem MUST pass the inner value through
//! `indexer::normalize_target_key` first (which rejects `..`,
//! absolute paths, NUL, backslash, and verifies the result stays
//! under `project_root`). The parser is purely syntactic.

use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypedRef {
    WorkItem(String),
    Plan(String),
    Adr(String),
    Pr(String),
    Path(PathBuf),
}

/// A path-shaped suffix contains `/` or ends in `.md`.
fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.ends_with(".md")
}

pub fn parse_typed_ref(raw: &str) -> Option<TypedRef> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Some(rest) = s.strip_prefix("work-item:") {
        if rest.is_empty() {
            return None;
        }
        if looks_like_path(rest) {
            return Some(TypedRef::Path(PathBuf::from(rest)));
        }
        return Some(TypedRef::WorkItem(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("plan:") {
        if rest.is_empty() {
            return None;
        }
        if looks_like_path(rest) {
            return Some(TypedRef::Path(PathBuf::from(rest)));
        }
        return Some(TypedRef::Plan(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("adr:") {
        if rest.is_empty() {
            return None;
        }
        return Some(TypedRef::Adr(rest.to_string()));
    }
    if let Some(rest) = s.strip_prefix("pr:") {
        if rest.is_empty() {
            return None;
        }
        return Some(TypedRef::Pr(rest.to_string()));
    }
    if looks_like_path(s) {
        return Some(TypedRef::Path(PathBuf::from(s)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_work_item_prefix() {
        assert_eq!(
            parse_typed_ref("work-item:0042"),
            Some(TypedRef::WorkItem("0042".into()))
        );
        assert_eq!(
            parse_typed_ref("work-item:PROJ-0042"),
            Some(TypedRef::WorkItem("PROJ-0042".into()))
        );
    }

    #[test]
    fn parses_plan_prefix() {
        assert_eq!(
            parse_typed_ref("plan:2026-05-31-0040-foo"),
            Some(TypedRef::Plan("2026-05-31-0040-foo".into()))
        );
    }

    #[test]
    fn parses_adr_and_pr_prefixes() {
        assert_eq!(
            parse_typed_ref("adr:0034"),
            Some(TypedRef::Adr("0034".into()))
        );
        assert_eq!(parse_typed_ref("pr:42"), Some(TypedRef::Pr("42".into())));
    }

    #[test]
    fn parses_repo_relative_path() {
        let r = parse_typed_ref("meta/plans/2026-05-31-0040-foo.md");
        assert_eq!(
            r,
            Some(TypedRef::Path(
                "meta/plans/2026-05-31-0040-foo.md".into()
            ))
        );
    }

    #[test]
    fn empty_and_unknown_return_none() {
        assert_eq!(parse_typed_ref(""), None);
        assert_eq!(parse_typed_ref("   "), None);
        assert_eq!(parse_typed_ref("nonsense"), None);
        assert_eq!(parse_typed_ref("foo"), None);
    }

    #[test]
    fn empty_typed_suffixes_return_none() {
        assert_eq!(parse_typed_ref("work-item:"), None);
        assert_eq!(parse_typed_ref("plan:"), None);
        assert_eq!(parse_typed_ref("adr:"), None);
        assert_eq!(parse_typed_ref("pr:"), None);
    }

    #[test]
    fn typed_prefix_with_path_payload_routes_to_path() {
        assert_eq!(
            parse_typed_ref("work-item:meta/work/0033-foo.md"),
            Some(TypedRef::Path("meta/work/0033-foo.md".into())),
        );
        assert_eq!(
            parse_typed_ref("plan:meta/plans/2026-05-31-0040-foo.md"),
            Some(TypedRef::Path("meta/plans/2026-05-31-0040-foo.md".into())),
        );
    }

    #[test]
    fn whitespace_is_trimmed() {
        assert_eq!(
            parse_typed_ref("  work-item:0042  "),
            Some(TypedRef::WorkItem("0042".into()))
        );
    }
}
