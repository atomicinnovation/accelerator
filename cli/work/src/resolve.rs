//! Input classification and the candidate-search cascade.

use crate::draft_id::DraftId;
use crate::id_pattern::DigitWidth;
use crate::id_pattern::IdPattern;
use crate::id_pattern::LiteralCase;
use crate::id_pattern::Walk;
use crate::id_scheme::IdScheme;
use crate::work_item_files::is_md_filename;

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

struct FullIdMatch<'a> {
    key: Option<&'a str>,
}

/// `input` read as a full ID under `pattern`: literals as written, and a
/// number of any width.
fn match_full_id<'a>(
    input: &'a str,
    pattern: &IdPattern,
) -> Option<FullIdMatch<'a>> {
    let walk = Walk {
        literals: LiteralCase::Exact,
        digits: DigitWidth::Any,
    };
    pattern
        .walk(input, walk)
        .map(|reading| FullIdMatch { key: reading.key })
}

/// Classifies a raw user input into the shape the search cascade dispatches
/// on.
#[must_use]
pub fn classify_input(input: &str, scheme: &IdScheme) -> InputClass {
    if input.is_empty() {
        return InputClass::Invalid;
    }
    if input.starts_with("./") || input.starts_with('/') || input.contains('/')
    {
        return InputClass::Path;
    }

    if DraftId::parse(input).is_some() {
        return InputClass::FullId;
    }
    let pattern = scheme.pattern();
    if pattern.is_tracker_owned() && tracker::is_tracker_key(input) {
        return InputClass::FullId;
    }

    let pattern_has_key = pattern.references_key();
    let all_digits = input.chars().all(|c| c.is_ascii_digit());

    if let Some(matched) = match_full_id(input, pattern) {
        if pattern_has_key && matched.key.is_some() {
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

fn is_draft_prefix(prefix: &str) -> bool {
    DraftId::PREFIX
        .strip_suffix('-')
        .is_some_and(|draft| prefix.eq_ignore_ascii_case(draft))
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
    scheme: &IdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let canonical = scheme.canonicalise(input).ok();
    let prefixes: Vec<String> = std::iter::once(input)
        .chain(canonical.as_deref())
        .map(|id| format!("{id}-"))
        .collect();
    let mut matches: Vec<String> = lister
        .filenames()
        .into_iter()
        .filter(|f| {
            is_md_filename(f)
                && prefixes.iter().any(|prefix| f.starts_with(prefix))
        })
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
    scheme: &IdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let filenames = lister.filenames();
    let pattern_has_key = scheme.pattern().references_key();
    let mut candidates: Vec<TaggedCandidate> = Vec::new();

    if pattern_has_key && scheme.key().is_some() {
        if let Ok(full_id) = scheme.canonicalise(input) {
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
        if let Some(legacy_padded) = IdScheme::pad_legacy_number(input) {
            let prefix = format!("{legacy_padded}-");
            for f in &filenames {
                if f.starts_with(&prefix) && is_md_filename(f) {
                    add_candidate(&mut candidates, f.clone(), "legacy");
                }
            }
        }
    }

    if !pattern_has_key {
        if let Ok(full_id) = scheme.canonicalise(input) {
            let prefix = format!("{full_id}-");
            for f in &filenames {
                if f.starts_with(&prefix) && is_md_filename(f) {
                    add_candidate(&mut candidates, f.clone(), "pattern-shape");
                }
            }
        }
    }

    if pattern_has_key {
        let width = scheme.pattern().width().unwrap_or(4);
        if let Ok(number) = input.parse::<u64>() {
            let padded = format!("{number:0width$}");
            let marker = format!("-{padded}-");
            for f in &filenames {
                let Some(marker_start) = f.find(&marker) else {
                    continue;
                };
                let prefix = &f[..marker_start];
                if !prefix.is_empty()
                    && !is_draft_prefix(prefix)
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
    scheme: &IdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    match class {
        SearchClass::FullId => resolve_full_id(input, scheme, lister),
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
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;
    use crate::id_scheme::TRACKER_TOKEN;

    struct FakeLister(Vec<&'static str>);

    impl super::DirectoryLister for FakeLister {
        fn filenames(&self) -> Vec<String> {
            self.0.iter().map(|s| (*s).to_owned()).collect()
        }
    }

    fn scheme(pattern: &str, key: Option<&str>) -> IdScheme {
        IdScheme::new(
            IdPattern::parse(pattern)
                .unwrap_or_else(|error| unreachable!("{error}")),
            key.map(|key| {
                IdKey::parse(key)
                    .unwrap_or_else(|error| unreachable!("{error}"))
            }),
        )
    }

    fn numeric() -> IdScheme {
        IdScheme::numeric()
    }

    fn project_scheme(default_project: Option<&str>) -> IdScheme {
        scheme("{project}-{number:04d}", default_project)
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
    fn classify_full_id_and_bare_number_under_key_pattern() {
        let scheme = scheme("{key}-{number:04d}", Some("PP"));
        assert_eq!(classify_input("PP-0042", &scheme), InputClass::FullId);
        assert_eq!(classify_input("42", &scheme), InputClass::BareNumber);
        assert_eq!(classify_input("abc", &scheme), InputClass::Invalid);
    }

    #[test]
    fn a_draft_id_is_not_classified_invalid_under_a_numeric_pattern() {
        assert_eq!(
            classify_input("draft-k7mq3x", &numeric()),
            InputClass::FullId
        );
        assert_eq!(
            classify_input("DRAFT-K7MQ3X", &numeric()),
            InputClass::FullId
        );
    }

    #[test]
    fn a_tracker_key_is_a_full_id_under_the_tracker_pattern() {
        let scheme = scheme(TRACKER_TOKEN, None);
        assert_eq!(classify_input("ENG-999", &scheme), InputClass::FullId);
        assert_eq!(classify_input("42", &scheme), InputClass::BareNumber);
        assert_eq!(classify_input("abc", &scheme), InputClass::Invalid);
    }

    #[test]
    fn bare_number_candidates_never_include_drafts() {
        let lister = FakeLister(vec!["PROJ-0007-a.md", "draft-0007-b.md"]);
        let scheme = project_scheme(None);
        let outcome = resolve("7", SearchClass::BareNumber, &scheme, &lister);
        assert_eq!(
            outcome,
            ResolveOutcome::Single("PROJ-0007-a.md".to_owned())
        );
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

    fn resolved_full_id(
        pattern: &str,
        key: Option<&str>,
        input: &str,
        filenames: Vec<&'static str>,
    ) -> ResolveOutcome {
        let scheme = scheme(pattern, key);
        assert_eq!(classify_input(input, &scheme), InputClass::FullId);
        resolve(input, SearchClass::FullId, &scheme, &FakeLister(filenames))
    }

    #[test]
    fn a_full_id_resolves_under_a_bare_number_token() {
        assert_eq!(
            resolved_full_id("a.{number}", None, "a.0042", vec!["a.0042-a.md"]),
            ResolveOutcome::Single("a.0042-a.md".to_owned())
        );
    }

    #[test]
    fn a_full_id_resolves_under_a_six_digit_key_pattern() {
        assert_eq!(
            resolved_full_id(
                "{key}-{number:06d}",
                Some("PP"),
                "PP-000042",
                vec!["PP-000042-a.md"]
            ),
            ResolveOutcome::Single("PP-000042-a.md".to_owned())
        );
    }

    #[test]
    fn a_short_full_id_resolves_to_its_padded_item() {
        assert_eq!(
            resolved_full_id(
                "{key}-{number:04d}",
                Some("PP"),
                "PROJ-42",
                vec!["PROJ-0042-x.md", "PROJ-0043-y.md"]
            ),
            ResolveOutcome::Single("PROJ-0042-x.md".to_owned())
        );
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
