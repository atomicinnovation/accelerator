//! Slug/path input classification and the candidate-search cascade for
//! `corpus resolve`.
//!
//! Pure string logic, mirroring the *shape* of `work::resolve`: a syntactic
//! [`InputClass`] classifier, a [`DirectoryLister`] port, and a [`resolve`]
//! cascade producing [`ResolveOutcome`]. The ambiguity rule is
//! corpus-specific — a `-<slug>.md` suffix across `YYYY-MM-DD-…` filenames for
//! a flat type, a directory-name slug for a set type — not
//! `work`'s number-prefix candidate sources. Filesystem access is the
//! adapter's, injected through [`DirectoryLister`], so the whole module is
//! unit-testable without a disk.

use crate::set::SetMatcher;

/// The syntactic shape of a raw `resolve` input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass {
    Path,
    Slug,
    Invalid,
}

/// How a doc type stores each document: a flat dated file, or a set
/// directory of the shape its matcher reads.
#[derive(Debug, Clone, Copy)]
pub enum TypeShape<'m> {
    Flat,
    Set(&'m SetMatcher),
}

/// Whether a path names a file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
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
/// Flat `.md` filenames for a [`TypeShape::Flat`] type; for a
/// [`TypeShape::Set`] type, the names of the directories that hold the
/// shape's root file. Injected so [`resolve`] never touches a filesystem
/// directly.
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

/// Recovers the descriptive slug of a set from its directory name: strips an
/// optional leading `YYYY-MM-DD` date and keeps the full remainder, else
/// takes the whole directory name.
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

fn set_candidates(
    slug: &str,
    lister: &dyn DirectoryLister,
) -> Vec<TaggedCandidate> {
    let mut candidates: Vec<TaggedCandidate> = lister
        .entries()
        .into_iter()
        .filter(|name| !name.starts_with('.'))
        .filter(|name| slug_from_set_dir(name) == slug)
        .map(|name| TaggedCandidate {
            path: name,
            tag: String::new(),
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    candidates
}

/// The set directory a path relative to the type directory lies in.
///
/// That is its first segment, when the segment is not dot-prefixed and the
/// rest of the path is nothing, the root, a member or a directory that may
/// hold members.
#[must_use]
pub fn set_directory_of<'p>(
    type_relative: &'p str,
    entry: EntryKind,
    matcher: &SetMatcher,
) -> Option<&'p str> {
    let (set_directory, within) = match type_relative.split_once('/') {
        Some((set_directory, within)) => (set_directory, Some(within)),
        None => (type_relative, None),
    };
    if set_directory.is_empty() || set_directory.starts_with('.') {
        return None;
    }
    let in_set = match (within, entry) {
        (None, EntryKind::Directory) => true,
        (None, EntryKind::File) => false,
        (Some(file), EntryKind::File) => matcher.classify(file).is_some(),
        (Some(directory), EntryKind::Directory) => {
            matcher.may_contain_members(directory)
        }
    };
    in_set.then_some(set_directory)
}

/// Resolves `slug` against the entries `lister` reports for a type of the given
/// `shape`, yielding a single match, tagged candidates for an ambiguity, or
/// nothing.
#[must_use]
pub fn resolve(
    slug: &str,
    shape: TypeShape<'_>,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome {
    let mut candidates = match shape {
        TypeShape::Flat => flat_candidates(slug, lister),
        TypeShape::Set(_) => set_candidates(slug, lister),
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
        classify_input, resolve, set_directory_of, slug_from_set_dir,
        DirectoryLister, EntryKind, InputClass, ResolveOutcome, TypeShape,
    };
    use crate::set::{
        MemberKind, MemberSlot, Presentation, SetDeclaration, SetMatcher,
        SetShape, SlotPatternError,
    };

    type Outcome = Result<(), (&'static str, SlotPatternError)>;

    const fn document(pattern: &'static str) -> MemberSlot {
        MemberSlot {
            pattern,
            kind: MemberKind::Document,
        }
    }

    static RESEARCHED: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "manifest.md",
            member_slots: &[
                document("brief.md"),
                document("findings/*.md"),
                document("findings/*.levels/*.md"),
                document("reports/*.md"),
            ],
        },
        presentation: Presentation::SetPage,
    };

    static PHOTOGRAPHED: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "inventory.md",
            member_slots: &[MemberSlot {
                pattern: "screenshots/*.png",
                kind: MemberKind::Asset,
            }],
        },
        presentation: Presentation::EntryPoint,
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

    fn resolve_set(
        slug: &str,
        entries: Vec<&'static str>,
    ) -> Result<ResolveOutcome, (&'static str, SlotPatternError)> {
        let matcher = SetMatcher::new(&RESEARCHED)?;
        Ok(resolve(
            slug,
            TypeShape::Set(&matcher),
            &FakeLister(entries),
        ))
    }

    #[test]
    fn a_set_slug_resolves_to_its_set_directory() -> Outcome {
        let outcome = resolve_set(
            "135214-current-app",
            vec![
                "2026-05-06-135214-current-app",
                "2026-05-21-004250-current-app",
            ],
        )?;
        assert_eq!(
            outcome,
            ResolveOutcome::Single("2026-05-06-135214-current-app".to_owned())
        );
        Ok(())
    }

    #[test]
    fn a_bare_set_slug_resolves_a_dateless_set_directory() -> Outcome {
        let outcome = resolve_set(
            "single-round-research",
            vec!["single-round-research"],
        )?;
        assert_eq!(
            outcome,
            ResolveOutcome::Single("single-round-research".to_owned())
        );
        Ok(())
    }

    #[test]
    fn a_set_slug_shared_by_two_directories_is_ambiguous() -> Outcome {
        let outcome = resolve_set(
            "current-app",
            vec!["2026-05-06-current-app", "2026-05-21-current-app"],
        )?;
        let ResolveOutcome::Ambiguous(candidates) = outcome else {
            unreachable!("expected an ambiguous outcome")
        };
        let paths: Vec<&str> =
            candidates.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, ["2026-05-06-current-app", "2026-05-21-current-app"]);
        Ok(())
    }

    #[test]
    fn an_absent_set_slug_is_not_found() -> Outcome {
        let outcome =
            resolve_set("no-such-set", vec!["2026-05-06-135214-current-app"])?;
        assert_eq!(outcome, ResolveOutcome::NotFound);
        Ok(())
    }

    #[test]
    fn a_dot_prefixed_set_directory_does_not_resolve_by_exact_name() -> Outcome
    {
        assert_eq!(
            resolve_set(".x.tmp", vec![".x.tmp"])?,
            ResolveOutcome::NotFound
        );
        Ok(())
    }

    #[test]
    fn a_dot_prefixed_set_directory_is_no_candidate() -> Outcome {
        assert_eq!(
            resolve_set("x", vec!["x", ".x"])?,
            ResolveOutcome::Single("x".to_owned())
        );
        Ok(())
    }

    fn set_of(
        declaration: &'static SetDeclaration,
        type_relative: &str,
        entry: EntryKind,
    ) -> Result<Option<String>, (&'static str, SlotPatternError)> {
        let matcher = SetMatcher::new(declaration)?;
        Ok(set_directory_of(type_relative, entry, &matcher).map(str::to_owned))
    }

    #[test]
    fn a_path_within_a_set_names_its_set_directory() -> Outcome {
        for (path, entry) in [
            ("s", EntryKind::Directory),
            ("s/manifest.md", EntryKind::File),
            ("s/findings/01-a.md", EntryKind::File),
            ("s/findings/01-a.levels/2-1.md", EntryKind::File),
            ("s/reports/r.md", EntryKind::File),
            ("s/findings", EntryKind::Directory),
            ("s/findings/01-a.levels", EntryKind::Directory),
        ] {
            assert_eq!(
                set_of(&RESEARCHED, path, entry)?.as_deref(),
                Some("s"),
                "{path}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_path_the_shape_does_not_hold_names_no_set() -> Outcome {
        for (path, entry) in [
            ("s/notes.txt", EntryKind::File),
            ("s/.conduct-run.json", EntryKind::File),
            ("s/findings/.01-x-web.md.invalid", EntryKind::File),
            ("s/findings/sub/x.md", EntryKind::File),
            ("s/.scratch", EntryKind::Directory),
            ("s/findings/01-a.levels/x", EntryKind::Directory),
            (".x.tmp", EntryKind::Directory),
            (".x.tmp/manifest.md", EntryKind::File),
            ("stray.md", EntryKind::File),
        ] {
            assert_eq!(set_of(&RESEARCHED, path, entry)?, None, "{path}");
        }
        Ok(())
    }

    #[test]
    fn a_slot_directory_names_its_set_only_when_the_shape_has_it() -> Outcome {
        assert_eq!(
            set_of(&PHOTOGRAPHED, "s/screenshots", EntryKind::Directory)?
                .as_deref(),
            Some("s")
        );
        assert_eq!(
            set_of(&PHOTOGRAPHED, "s/prototype-full/x.html", EntryKind::File)?,
            None
        );
        assert_eq!(
            set_of(&PHOTOGRAPHED, "s/prototype-full", EntryKind::Directory)?,
            None
        );
        Ok(())
    }
}
