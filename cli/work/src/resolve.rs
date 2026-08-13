//! Input classification and the candidate-search cascade.
//!
//! Both stages avoid `corpus_adapters::work_item_pattern` (which needs the
//! `regex` crate) deliberately: `work`'s own import-restriction rule
//! (`work_domain_imports_only_permitted` in `cli/pup.ron`) keeps this crate
//! dependency-free, so pattern matching here is a small, regex-free,
//! greedy token-walker — sufficient for the DSL's own grammar, since
//! adjacent dynamic tokens are already forbidden by the pattern grammar,
//! so a digit or letter run never needs to backtrack against what follows
//! it.

use corpus::WorkItemIdScheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass {
    Path,
    FullId,
    BareNumber,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchClass {
    FullId,
    BareNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedCandidate {
    pub path: String,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveOutcome {
    Single(String),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound,
}

/// Lists the candidate filenames in the configured work directory. Injected
/// so `resolve` never touches a filesystem directly.
pub trait DirectoryLister {
    fn filenames(&self) -> Vec<String>;
}

enum Segment {
    Literal(String),
    Project,
    Number,
}

/// A regex-free, best-effort walk of `pattern`'s tokens into literal and
/// dynamic segments. Malformed tokens collapse to an empty segment list
/// (nothing can ever match), matching how an already-invalid pattern would
/// never reach this code in practice (callers validate the pattern first).
fn segments(pattern: &str) -> Vec<Segment> {
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut out = Vec::new();
    let mut literal = String::new();
    let mut i = 0;
    while i < len {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();
        if ch == '{' && next == Some('{') {
            literal.push('{');
            i += 2;
            continue;
        }
        if ch == '}' && next == Some('}') {
            literal.push('}');
            i += 2;
            continue;
        }
        if ch == '{' {
            let Some(close_offset) =
                chars[i + 1..].iter().position(|&c| c == '}')
            else {
                return Vec::new();
            };
            let close = i + 1 + close_offset;
            let token: String = chars[i + 1..close].iter().collect();
            if !literal.is_empty() {
                out.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            if token == "project" {
                out.push(Segment::Project);
            } else if token == "number" || token.starts_with("number:") {
                out.push(Segment::Number);
            } else {
                return Vec::new();
            }
            i = close + 1;
            continue;
        }
        literal.push(ch);
        i += 1;
    }
    if !literal.is_empty() {
        out.push(Segment::Literal(literal));
    }
    out
}

struct FullIdMatch {
    project: Option<String>,
}

/// Greedily matches `input` against `pattern`'s segments. A `{project}`
/// segment consumes the maximal `[A-Za-z][A-Za-z0-9]*` run; a `{number...}`
/// segment consumes the maximal digit run; a literal segment must match
/// exactly. No backtracking — sufficient given the DSL's own grammar (see
/// module docs).
fn match_full_id(input: &str, pattern: &str) -> Option<FullIdMatch> {
    let segments = segments(pattern);
    if segments.is_empty() {
        return None;
    }
    let mut rest = input;
    let mut project = None;
    let mut number = None;
    for segment in &segments {
        match segment {
            Segment::Literal(text) => {
                rest = rest.strip_prefix(text.as_str())?;
            }
            Segment::Project => {
                let mut end = 0;
                for (index, c) in rest.char_indices() {
                    let ok = if index == 0 {
                        c.is_ascii_alphabetic()
                    } else {
                        c.is_ascii_alphanumeric()
                    };
                    if !ok {
                        break;
                    }
                    end = index + c.len_utf8();
                }
                if end == 0 {
                    return None;
                }
                project = Some(rest[..end].to_owned());
                rest = &rest[end..];
            }
            Segment::Number => {
                let end = rest
                    .char_indices()
                    .take_while(|(_, c)| c.is_ascii_digit())
                    .last()
                    .map_or(0, |(index, c)| index + c.len_utf8());
                if end == 0 {
                    return None;
                }
                number = Some(rest[..end].to_owned());
                rest = &rest[end..];
            }
        }
    }
    if !rest.is_empty() || number.is_none() {
        return None;
    }
    Some(FullIdMatch { project })
}

/// Classifies `input` as one of path / full ID / bare number / invalid.
/// Classifies a raw user input into the shape the search cascade dispatches
/// on.
#[must_use]
pub fn classify_input(input: &str, scheme: &WorkItemIdScheme) -> InputClass {
    if input.is_empty() {
        return InputClass::Invalid;
    }
    if input.starts_with("./") || input.starts_with('/') || input.contains('/')
    {
        return InputClass::Path;
    }

    let pattern_has_project = scheme.id_pattern.contains("{project}");
    let all_digits = input.chars().all(|c| c.is_ascii_digit());

    if let Some(matched) = match_full_id(input, &scheme.id_pattern) {
        if pattern_has_project
            && matched.project.as_deref().is_some_and(|p| !p.is_empty())
        {
            return InputClass::FullId;
        }
        if all_digits {
            return InputClass::BareNumber;
        }
        return InputClass::FullId;
    }
    if all_digits {
        return InputClass::BareNumber;
    }
    InputClass::Invalid
}

fn is_md_filename(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        && name.contains('.')
}

fn width_with_default(scheme: &WorkItemIdScheme) -> usize {
    let width = scheme.canonical_digit_width();
    if width == 0 {
        4
    } else {
        width
    }
}

fn add_candidate(
    candidates: &mut Vec<TaggedCandidate>,
    path: String,
    tag: &str,
) {
    if candidates.iter().any(|c| c.path == path) {
        return;
    }
    candidates.push(TaggedCandidate {
        path,
        tag: tag.to_owned(),
    });
}

fn resolve_full_id(
    input: &str,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let prefix = format!("{input}-");
    let mut matches: Vec<String> = lister
        .filenames()
        .into_iter()
        .filter(|f| f.starts_with(&prefix) && is_md_filename(f))
        .collect();
    matches.sort();
    match matches.len() {
        0 => ResolveOutcome::NotFound,
        1 => ResolveOutcome::Single(
            matches.into_iter().next().unwrap_or_default(),
        ),
        _ => ResolveOutcome::Ambiguous(
            matches
                .into_iter()
                .map(|path| TaggedCandidate {
                    path,
                    tag: String::new(),
                })
                .collect(),
        ),
    }
}

fn resolve_bare_number(
    input: &str,
    scheme: &WorkItemIdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let filenames = lister.filenames();
    let pattern_has_project = scheme.id_pattern.contains("{project}");
    let mut candidates: Vec<TaggedCandidate> = Vec::new();

    if pattern_has_project && scheme.default_project_code.is_some() {
        if let Some(full_id) = scheme.canonicalise_id(input) {
            let prefix = format!("{full_id}-");
            for f in &filenames {
                if f.starts_with(&prefix) && is_md_filename(f) {
                    add_candidate(
                        &mut candidates,
                        f.clone(),
                        "project-prepended",
                    );
                }
            }
        }
    }

    if input.len() <= 4 {
        if let Some(legacy_padded) = WorkItemIdScheme::pad_legacy_number(input)
        {
            let prefix = format!("{legacy_padded}-");
            for f in &filenames {
                if f.starts_with(&prefix) && is_md_filename(f) {
                    add_candidate(&mut candidates, f.clone(), "legacy");
                }
            }
        }
    }

    if !pattern_has_project {
        if let Some(full_id) = scheme.canonicalise_id(input) {
            let prefix = format!("{full_id}-");
            for f in &filenames {
                if f.starts_with(&prefix) && is_md_filename(f) {
                    add_candidate(&mut candidates, f.clone(), "pattern-shape");
                }
            }
        }
    }

    if pattern_has_project {
        let width = width_with_default(scheme);
        if let Ok(number) = input.parse::<u64>() {
            let padded = format!("{number:0width$}");
            let marker = format!("-{padded}-");
            for f in &filenames {
                let Some(marker_start) = f.find(&marker) else {
                    continue;
                };
                let prefix = &f[..marker_start];
                if !prefix.is_empty()
                    && prefix
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
                    && prefix.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    add_candidate(&mut candidates, f.clone(), prefix);
                }
            }
        }
    }

    match candidates.len() {
        0 => ResolveOutcome::NotFound,
        1 => ResolveOutcome::Single(
            candidates
                .into_iter()
                .next()
                .map(|c| c.path)
                .unwrap_or_default(),
        ),
        _ => ResolveOutcome::Ambiguous(candidates),
    }
}

/// The candidate-search cascade for `FullId`/`BareNumber`-classified input.
///
/// `resolve` is infallible: its caller guarantees `class` is one of the two
/// variants it handles — `Path`/`Invalid` need no candidate search and are
/// handled entirely by the CLI layer before `resolve` is ever called.
#[must_use]
pub fn resolve(
    input: &str,
    class: SearchClass,
    scheme: &WorkItemIdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    match class {
        SearchClass::FullId => resolve_full_id(input, lister),
        SearchClass::BareNumber => resolve_bare_number(input, scheme, lister),
    }
}

#[cfg(test)]
// DSL pattern strings such as "{project}" are literal token markers, not
// format args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::{
        classify_input, resolve, InputClass, ResolveOutcome, SearchClass,
    };
    use corpus::WorkItemIdScheme;

    struct FakeLister(Vec<&'static str>);

    impl super::DirectoryLister for FakeLister {
        fn filenames(&self) -> Vec<String> {
            self.0.iter().map(|s| (*s).to_owned()).collect()
        }
    }

    fn numeric() -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: "{number:04d}".to_owned(),
            default_project_code: None,
        }
    }

    fn project_scheme(default_project: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: "{project}-{number:04d}".to_owned(),
            default_project_code: default_project.map(str::to_owned),
        }
    }

    #[test]
    fn classify_bare_number() {
        assert_eq!(classify_input("42", &numeric()), InputClass::BareNumber);
        assert_eq!(classify_input("0042", &numeric()), InputClass::BareNumber);
    }

    #[test]
    fn classify_invalid() {
        assert_eq!(classify_input("abc", &numeric()), InputClass::Invalid);
    }

    #[test]
    fn classify_path() {
        assert_eq!(classify_input("./foo.md", &numeric()), InputClass::Path);
        assert_eq!(classify_input("foo/bar.md", &numeric()), InputClass::Path);
        assert_eq!(classify_input("/abs/foo.md", &numeric()), InputClass::Path);
    }

    #[test]
    fn classify_full_id_and_bare_number_under_project_pattern() {
        let scheme = project_scheme(None);
        assert_eq!(classify_input("PROJ-0042", &scheme), InputClass::FullId);
        assert_eq!(classify_input("42", &scheme), InputClass::BareNumber);
        assert_eq!(classify_input("abc", &scheme), InputClass::Invalid);
    }

    #[test]
    fn resolve_full_id_single_match() {
        let lister = FakeLister(vec!["0042-foo.md"]);
        let outcome = resolve("0042", SearchClass::FullId, &numeric(), &lister);
        assert_eq!(outcome, ResolveOutcome::Single("0042-foo.md".to_owned()));
    }

    #[test]
    fn resolve_full_id_ambiguous() {
        let lister = FakeLister(vec!["0042-foo.md", "0042-bar.md"]);
        let outcome = resolve("0042", SearchClass::FullId, &numeric(), &lister);
        let ResolveOutcome::Ambiguous(candidates) = outcome else {
            unreachable!("expected ambiguous outcome")
        };
        let mut paths: Vec<&str> =
            candidates.iter().map(|c| c.path.as_str()).collect();
        paths.sort_unstable();
        assert_eq!(paths, vec!["0042-bar.md", "0042-foo.md"]);
    }

    #[test]
    fn resolve_bare_number_not_found() {
        let lister = FakeLister(vec![]);
        let outcome =
            resolve("99", SearchClass::BareNumber, &numeric(), &lister);
        assert_eq!(outcome, ResolveOutcome::NotFound);
    }

    #[test]
    fn resolve_bare_number_all_four_sources_simultaneously() {
        let lister =
            FakeLister(vec!["PROJ-0007-a.md", "0007-b.md", "OTHER-0007-c.md"]);
        let scheme = project_scheme(Some("PROJ"));
        let outcome = resolve("7", SearchClass::BareNumber, &scheme, &lister);
        let ResolveOutcome::Ambiguous(candidates) = outcome else {
            unreachable!("expected ambiguous outcome")
        };
        assert_eq!(
            candidates,
            vec![
                super::TaggedCandidate {
                    path: "PROJ-0007-a.md".to_owned(),
                    tag: "project-prepended".to_owned(),
                },
                super::TaggedCandidate {
                    path: "0007-b.md".to_owned(),
                    tag: "legacy".to_owned(),
                },
                super::TaggedCandidate {
                    path: "OTHER-0007-c.md".to_owned(),
                    tag: "OTHER".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn resolve_bare_number_single_cross_project_match() {
        let lister = FakeLister(vec!["OTHER-0007-c.md"]);
        let scheme = project_scheme(None);
        let outcome = resolve("7", SearchClass::BareNumber, &scheme, &lister);
        assert_eq!(
            outcome,
            ResolveOutcome::Single("OTHER-0007-c.md".to_owned())
        );
    }
}
