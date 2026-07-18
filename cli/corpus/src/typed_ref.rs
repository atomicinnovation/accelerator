//! Parser for typed-linkage reference values.
//!
//! **Path safety**: [`TypedRef::Path`] carries the raw, unvalidated
//! repo-relative path string. A caller that resolves it against the filesystem
//! must first reject `..`, absolute paths, NUL, and backslash and confirm the
//! result stays under the project root. This parser is purely syntactic.

use std::path::PathBuf;

/// A parsed typed-linkage reference.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypedRef {
    WorkItem(String),
    Plan(String),
    Adr(String),
    Pr(String),
    Path(PathBuf),
}

fn looks_like_path(value: &str) -> bool {
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    {
        value.contains('/') || value.ends_with(".md")
    }
}

/// Parses a single typed-linkage reference value, or `None` when the string is
/// empty or matches no known form.
#[must_use]
pub fn parse_typed_ref(raw: &str) -> Option<TypedRef> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("work-item:") {
        if rest.is_empty() {
            return None;
        }
        if looks_like_path(rest) {
            return Some(TypedRef::Path(PathBuf::from(rest)));
        }
        return Some(TypedRef::WorkItem(rest.to_owned()));
    }
    if let Some(rest) = trimmed.strip_prefix("plan:") {
        if rest.is_empty() {
            return None;
        }
        if looks_like_path(rest) {
            return Some(TypedRef::Path(PathBuf::from(rest)));
        }
        return Some(TypedRef::Plan(rest.to_owned()));
    }
    if let Some(rest) = trimmed.strip_prefix("adr:") {
        if rest.is_empty() {
            return None;
        }
        return Some(TypedRef::Adr(rest.to_owned()));
    }
    if let Some(rest) = trimmed.strip_prefix("pr:") {
        if rest.is_empty() {
            return None;
        }
        return Some(TypedRef::Pr(rest.to_owned()));
    }
    if looks_like_path(trimmed) {
        return Some(TypedRef::Path(PathBuf::from(trimmed)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{parse_typed_ref, TypedRef};

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
        assert_eq!(
            parse_typed_ref("meta/plans/2026-05-31-0040-foo.md"),
            Some(TypedRef::Path("meta/plans/2026-05-31-0040-foo.md".into()))
        );
    }

    #[test]
    fn empty_and_unknown_return_none() {
        assert_eq!(parse_typed_ref(""), None);
        assert_eq!(parse_typed_ref("   "), None);
        assert_eq!(parse_typed_ref("nonsense"), None);
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
