//! Slug/path input classification and the candidate-search cascade for
//! `corpus resolve`.
//!
//! Pure string logic, mirroring the *shape* of `work::resolve`: a syntactic
//! [`InputClass`] classifier, a [`DirectoryLister`] port, and a [`resolve`]
//! cascade producing [`ResolveOutcome`]. The ambiguity rule is
//! corpus-specific — a `-<slug>.md` suffix across `YYYY-MM-DD-…` filenames for
//! a flat type, a directory-name slug for a nested-manifest type — not
//! `work`'s number-prefix candidate sources. Filesystem access is the
//! adapter's, injected through [`DirectoryLister`], so the whole module is
//! unit-testable without a disk.

/// The syntactic shape of a raw `resolve` input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass {
    Path,
    Slug,
    Invalid,
}

/// How a doc type stores each document: a flat dated file, or a
/// nested-manifest set directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeShape {
    Flat,
    NestedManifest,
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

/// Lists the candidate entries in the configured type directory.
///
/// Flat `.md` filenames for a [`TypeShape::Flat`] type, immediate set-directory
/// names for a [`TypeShape::NestedManifest`] type. Injected so [`resolve`]
/// never touches a filesystem directly.
pub trait DirectoryLister {
    fn entries(&self) -> Vec<String>;
}

/// Classifies `input` as a path, a slug, or invalid.
///
/// A `/`-bearing input is a path the adapter narrows against the filesystem; an
/// empty input is invalid; anything else is a slug the candidate search
/// dispatches on.
#[must_use]
pub fn classify_input(input: &str) -> InputClass {
    if input.trim().is_empty() {
        return InputClass::Invalid;
    }
    if input.starts_with("./") || input.starts_with('/') || input.contains('/')
    {
        return InputClass::Path;
    }
    InputClass::Slug
}

/// Recovers the descriptive slug of a nested-manifest set from its directory
/// name: strips an optional leading `YYYY-MM-DD` date and keeps the full
/// remainder, else takes the whole directory name.
///
/// Deliberately not [`crate::slug::derive`], which requires a `.md` filename
/// and whose date/id stripping would mis-read a 6-digit `HHMMSS` run as a
/// work-item id.
#[must_use]
pub fn slug_from_set_dir(dir_name: &str) -> String {
    strip_iso_date_prefix(dir_name)
        .unwrap_or(dir_name)
        .to_owned()
}

fn is_iso_date_prefixed(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 11
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && bytes[10] == b'-'
}

fn strip_iso_date_prefix(name: &str) -> Option<&str> {
    is_iso_date_prefixed(name).then(|| &name[11..])
}

fn flat_candidates(
    slug: &str,
    lister: &dyn DirectoryLister,
) -> Vec<TaggedCandidate> {
    let suffix = format!("-{slug}.md");
    let mut candidates: Vec<TaggedCandidate> = lister
        .entries()
        .into_iter()
        .filter(|name| is_iso_date_prefixed(name) && name.ends_with(&suffix))
        .map(|name| {
            let tag = name.get(0..10).unwrap_or_default().to_owned();
            TaggedCandidate { path: name, tag }
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    candidates
}

fn nested_candidates(
    slug: &str,
    lister: &dyn DirectoryLister,
) -> Vec<TaggedCandidate> {
    let mut candidates: Vec<TaggedCandidate> = lister
        .entries()
        .into_iter()
        .filter(|name| slug_from_set_dir(name) == slug)
        .map(|name| TaggedCandidate {
            path: name,
            tag: String::new(),
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    candidates
}

/// Resolves `slug` against the entries `lister` reports for a type of the given
/// `shape`, yielding a single match, tagged candidates for an ambiguity, or
/// nothing.
#[must_use]
pub fn resolve(
    slug: &str,
    shape: TypeShape,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let mut candidates = match shape {
        TypeShape::Flat => flat_candidates(slug, lister),
        TypeShape::NestedManifest => nested_candidates(slug, lister),
    };
    match candidates.len() {
        0 => ResolveOutcome::NotFound,
        1 => ResolveOutcome::Single(candidates.remove(0).path),
        _ => ResolveOutcome::Ambiguous(candidates),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_input, resolve, slug_from_set_dir, DirectoryLister,
        InputClass, ResolveOutcome, TypeShape,
    };

    struct FakeLister(Vec<&'static str>);

    impl DirectoryLister for FakeLister {
        fn entries(&self) -> Vec<String> {
            self.0.iter().map(|entry| (*entry).to_owned()).collect()
        }
    }

    #[test]
    fn classifies_a_bare_slug() {
        assert_eq!(classify_input("current-app"), InputClass::Slug);
        assert_eq!(classify_input("135214-current-app"), InputClass::Slug);
    }

    #[test]
    fn classifies_a_slashed_input_as_a_path() {
        assert_eq!(classify_input("./foo"), InputClass::Path);
        assert_eq!(classify_input("meta/research/topics/x"), InputClass::Path);
        assert_eq!(classify_input("/abs/x"), InputClass::Path);
    }

    #[test]
    fn classifies_an_empty_input_as_invalid() {
        assert_eq!(classify_input(""), InputClass::Invalid);
        assert_eq!(classify_input("   "), InputClass::Invalid);
    }

    #[test]
    fn a_dated_set_directory_name_yields_the_hhmmss_id_slug() {
        assert_eq!(
            slug_from_set_dir("2026-05-06-135214-current-app"),
            "135214-current-app"
        );
    }

    #[test]
    fn a_bare_set_directory_name_yields_itself() {
        assert_eq!(
            slug_from_set_dir("single-round-research"),
            "single-round-research"
        );
    }

    #[test]
    fn a_flat_slug_resolves_to_its_single_dated_file() {
        let lister = FakeLister(vec![
            "2026-09-08-0277-single-round-web-research-engine.md",
            "2026-01-01-unrelated.md",
        ]);
        let outcome = resolve(
            "0277-single-round-web-research-engine",
            TypeShape::Flat,
            &lister,
        );
        assert_eq!(
            outcome,
            ResolveOutcome::Single(
                "2026-09-08-0277-single-round-web-research-engine.md"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_flat_slug_shared_across_dates_is_ambiguous_with_date_tags() {
        let lister = FakeLister(vec![
            "2026-09-09-0277-single-round-web-research-engine.md",
            "2026-09-08-0277-single-round-web-research-engine.md",
        ]);
        let outcome = resolve(
            "0277-single-round-web-research-engine",
            TypeShape::Flat,
            &lister,
        );
        let ResolveOutcome::Ambiguous(candidates) = outcome else {
            unreachable!("expected an ambiguous outcome")
        };
        let tags: Vec<&str> =
            candidates.iter().map(|c| c.tag.as_str()).collect();
        assert_eq!(tags, vec!["2026-09-08", "2026-09-09"]);
    }

    #[test]
    fn an_absent_flat_slug_is_not_found() {
        let lister = FakeLister(vec!["2026-09-08-something-else.md"]);
        let outcome = resolve("nothing-here", TypeShape::Flat, &lister);
        assert_eq!(outcome, ResolveOutcome::NotFound);
    }

    #[test]
    fn a_nested_slug_resolves_to_its_set_directory() {
        let lister = FakeLister(vec![
            "2026-05-06-135214-current-app",
            "2026-05-21-004250-current-app",
        ]);
        let outcome =
            resolve("135214-current-app", TypeShape::NestedManifest, &lister);
        assert_eq!(
            outcome,
            ResolveOutcome::Single("2026-05-06-135214-current-app".to_owned())
        );
    }

    #[test]
    fn a_bare_nested_slug_resolves_a_dateless_set_directory() {
        let lister = FakeLister(vec!["single-round-research"]);
        let outcome = resolve(
            "single-round-research",
            TypeShape::NestedManifest,
            &lister,
        );
        assert_eq!(
            outcome,
            ResolveOutcome::Single("single-round-research".to_owned())
        );
    }

    #[test]
    fn an_absent_nested_slug_is_not_found() {
        let lister = FakeLister(vec!["2026-05-06-135214-current-app"]);
        let outcome =
            resolve("no-such-set", TypeShape::NestedManifest, &lister);
        assert_eq!(outcome, ResolveOutcome::NotFound);
    }
}
