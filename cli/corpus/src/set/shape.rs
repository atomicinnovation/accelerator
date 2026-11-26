use std::fmt;

use crate::set::MemberKind;
use crate::set::Presentation;
use crate::set::SetDeclaration;
use crate::set::SetShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPlace {
    Root,
    Member(MemberKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotPatternError {
    AbsolutePath,
    EmptySegment,
    ParentSegment,
    RecursiveWildcard,
}

impl fmt::Display for SlotPatternError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AbsolutePath => "a slot is relative to its set directory",
            Self::EmptySegment => "a slot has no empty segment",
            Self::ParentSegment => "a slot stays inside its set directory",
            Self::RecursiveWildcard => "a slot's wildcard spans one segment",
        })
    }
}

impl std::error::Error for SlotPatternError {}

/// A parsed member slot: a path of segments relative to the set directory,
/// each a name in which `*` stands for any run of characters.
#[derive(Debug, Clone)]
pub struct SlotPattern {
    directories: Vec<NamePattern>,
    file: FilePattern,
}

/// A segment's name pattern. A `*` never spans `/`, and never matches a name
/// that starts with a dot, so dot-prefixed working files fill no slot.
#[derive(Debug, Clone)]
struct NamePattern(String);

/// The final segment, whose extension is matched in any case because
/// filesystems and tools disagree on an extension's case, while every other
/// name is matched exactly.
#[derive(Debug, Clone)]
struct FilePattern {
    stem: NamePattern,
    extension: Option<String>,
}

impl SlotPattern {
    /// # Errors
    ///
    /// A [`SlotPatternError`] when the pattern is absolute, has an empty
    /// segment, leaves the set directory or spans segments with `**`.
    pub fn parse(pattern: &str) -> Result<Self, SlotPatternError> {
        if pattern.starts_with('/') {
            return Err(SlotPatternError::AbsolutePath);
        }
        let (directories, file) = match pattern.rsplit_once('/') {
            Some((directories, file)) => (
                directories
                    .split('/')
                    .map(|segment| parsed(segment).map(NamePattern))
                    .collect::<Result<_, _>>()?,
                file,
            ),
            None => (Vec::new(), pattern),
        };
        Ok(Self {
            directories,
            file: FilePattern::of(parsed(file)?),
        })
    }

    fn matches(&self, segments: &[&str]) -> bool {
        segments.split_last().is_some_and(|(file, directories)| {
            directories.len() == self.directories.len()
                && self.leads_into(directories)
                && self.file.matches(file)
        })
    }

    fn leads_into(&self, directories: &[&str]) -> bool {
        directories.len() <= self.directories.len()
            && self
                .directories
                .iter()
                .zip(directories)
                .all(|(pattern, name)| pattern.matches(name))
    }
}

fn parsed(segment: &str) -> Result<String, SlotPatternError> {
    match segment {
        "" => Err(SlotPatternError::EmptySegment),
        ".." => Err(SlotPatternError::ParentSegment),
        _ if segment.contains("**") => Err(SlotPatternError::RecursiveWildcard),
        _ => Ok(segment.to_owned()),
    }
}

impl NamePattern {
    fn matches(&self, name: &str) -> bool {
        let Some((prefix, after_first_wildcard)) = self.0.split_once('*')
        else {
            return self.0 == name;
        };
        if prefix.is_empty() && name.starts_with('.') {
            return false;
        }
        let Some(mut rest) = name.strip_prefix(prefix) else {
            return false;
        };
        let mut pieces: Vec<&str> = after_first_wildcard.split('*').collect();
        let suffix = pieces.pop().unwrap_or_default();
        for piece in pieces {
            let Some(at) = rest.find(piece) else {
                return false;
            };
            rest = &rest[at + piece.len()..];
        }
        rest.ends_with(suffix)
    }
}

impl FilePattern {
    fn of(segment: String) -> Self {
        match segment.rsplit_once('.') {
            Some((stem, extension)) if !extension.contains('*') => Self {
                stem: NamePattern(stem.to_owned()),
                extension: Some(extension.to_owned()),
            },
            _ => Self {
                stem: NamePattern(segment),
                extension: None,
            },
        }
    }

    fn matches(&self, name: &str) -> bool {
        let Some(extension) = &self.extension else {
            return self.stem.matches(name);
        };
        name.rsplit_once('.').is_some_and(|(stem, actual)| {
            actual.eq_ignore_ascii_case(extension)
                && !name.starts_with('.')
                && self.stem.matches(stem)
        })
    }
}

/// A set declaration with its slots parsed, answering where a path sits in a
/// set of its shape.
#[derive(Debug, Clone)]
pub struct SetMatcher {
    declaration: &'static SetDeclaration,
    slots: Vec<(SlotPattern, MemberKind)>,
}

impl SetMatcher {
    pub(crate) fn compiled(
        declaration: &'static SetDeclaration,
    ) -> Result<Self, (&'static str, SlotPatternError)> {
        let slots = declaration
            .shape
            .member_slots
            .iter()
            .map(|slot| {
                SlotPattern::parse(slot.pattern)
                    .map(|pattern| (pattern, slot.kind))
                    .map_err(|cause| (slot.pattern, cause))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { declaration, slots })
    }

    #[must_use]
    pub const fn shape(&self) -> &'static SetShape {
        &self.declaration.shape
    }

    #[must_use]
    pub const fn presentation(&self) -> Presentation {
        self.declaration.presentation
    }

    #[must_use]
    pub const fn root(&self) -> &'static str {
        self.declaration.shape.root
    }

    #[must_use]
    pub fn classify(&self, set_relative: &str) -> Option<SetPlace> {
        if set_relative == self.root() {
            return Some(SetPlace::Root);
        }
        let segments: Vec<&str> = set_relative.split('/').collect();
        self.slots
            .iter()
            .find(|(pattern, _)| pattern.matches(&segments))
            .map(|(_, kind)| SetPlace::Member(*kind))
    }

    /// Whether a set-relative directory can hold a member, at any depth.
    #[must_use]
    pub fn may_contain_members(&self, set_relative_dir: &str) -> bool {
        let directories: Vec<&str> = set_relative_dir.split('/').collect();
        self.slots
            .iter()
            .any(|(pattern, _)| pattern.leads_into(&directories))
    }
}

#[cfg(feature = "test-support")]
impl SetMatcher {
    /// # Errors
    ///
    /// The first slot pattern that does not parse, with why.
    pub fn new(
        declaration: &'static SetDeclaration,
    ) -> Result<Self, (&'static str, SlotPatternError)> {
        Self::compiled(declaration)
    }
}

#[cfg(test)]
mod tests {
    use super::{SetMatcher, SetPlace, SlotPattern, SlotPatternError};
    use crate::set::{
        MemberKind, MemberSlot, Presentation, SetDeclaration, SetShape,
    };

    static INDEXED: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "index.md",
            member_slots: &[
                MemberSlot {
                    pattern: "docs/*.md",
                    kind: MemberKind::Document,
                },
                MemberSlot {
                    pattern: "docs/*.parts/*.md",
                    kind: MemberKind::Document,
                },
                MemberSlot {
                    pattern: "img/*.png",
                    kind: MemberKind::Asset,
                },
            ],
        },
        presentation: Presentation::SetPage,
    };

    static PHOTOGRAPHED: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "index.md",
            member_slots: &[MemberSlot {
                pattern: "img/*.jpg",
                kind: MemberKind::Asset,
            }],
        },
        presentation: Presentation::EntryPoint,
    };

    static WILD: SetDeclaration = SetDeclaration {
        shape: SetShape {
            root: "index.md",
            member_slots: &[MemberSlot {
                pattern: "*",
                kind: MemberKind::Document,
            }],
        },
        presentation: Presentation::SetPage,
    };

    type Outcome = Result<(), (&'static str, SlotPatternError)>;

    fn place_in(
        declaration: &'static SetDeclaration,
        set_relative: &str,
    ) -> Result<Option<SetPlace>, (&'static str, SlotPatternError)> {
        Ok(SetMatcher::new(declaration)?.classify(set_relative))
    }

    const DOCUMENT: Option<SetPlace> =
        Some(SetPlace::Member(MemberKind::Document));
    const ASSET: Option<SetPlace> = Some(SetPlace::Member(MemberKind::Asset));

    #[test]
    fn parses_literal_and_single_segment_wildcard_slots() {
        for pattern in [
            "brief.md",
            "findings/*.md",
            "findings/*.levels/*.md",
            "screenshots/*.png",
        ] {
            assert!(SlotPattern::parse(pattern).is_ok(), "{pattern}");
        }
    }

    #[test]
    fn refuses_slots_that_span_segments_or_leave_the_set() {
        assert_eq!(
            SlotPattern::parse("findings/**/*.md").err(),
            Some(SlotPatternError::RecursiveWildcard)
        );
        assert_eq!(
            SlotPattern::parse("findings//x.md").err(),
            Some(SlotPatternError::EmptySegment)
        );
        assert_eq!(
            SlotPattern::parse("/findings/x.md").err(),
            Some(SlotPatternError::AbsolutePath)
        );
        assert_eq!(
            SlotPattern::parse("../x.md").err(),
            Some(SlotPatternError::ParentSegment)
        );
    }

    #[test]
    fn a_wildcard_matches_any_name_that_does_not_start_with_a_dot() -> Outcome {
        for name in ["01-a", "Some File (v2)", "01-a.levels"] {
            assert_eq!(place_in(&WILD, name)?, DOCUMENT, "{name}");
        }
        for name in [".x", ".01-x-web.md.invalid", ".conduct-run.json"] {
            assert_eq!(place_in(&WILD, name)?, None, "{name}");
        }
        assert_eq!(place_in(&WILD, "a/b")?, None);
        Ok(())
    }

    #[test]
    fn classifies_the_root_and_each_kind_of_member() -> Outcome {
        assert_eq!(place_in(&INDEXED, "index.md")?, Some(SetPlace::Root));
        assert_eq!(place_in(&INDEXED, "docs/a.md")?, DOCUMENT);
        assert_eq!(place_in(&INDEXED, "docs/a.parts/1.md")?, DOCUMENT);
        assert_eq!(place_in(&INDEXED, "img/a.png")?, ASSET);
        Ok(())
    }

    #[test]
    fn a_path_that_fills_no_slot_is_not_in_the_set() -> Outcome {
        for path in
            ["docs/sub/x.md", "notes.txt", "img/sub/a.png", ".hidden.md"]
        {
            assert_eq!(place_in(&INDEXED, path)?, None, "{path}");
        }
        Ok(())
    }

    #[test]
    fn an_extension_matches_in_any_case() -> Outcome {
        assert_eq!(place_in(&INDEXED, "img/a.PNG")?, ASSET);
        assert_eq!(place_in(&INDEXED, "img/a.Png")?, ASSET);
        assert_eq!(place_in(&INDEXED, "docs/a.MD")?, DOCUMENT);
        assert_eq!(place_in(&PHOTOGRAPHED, "img/b.JPG")?, ASSET);
        Ok(())
    }

    #[test]
    fn a_directory_name_matches_only_in_its_declared_case() -> Outcome {
        for path in ["Img/a.png", "IMG/a.png", "docs/a.PARTS/1.md", "Docs/a.md"]
        {
            assert_eq!(place_in(&INDEXED, path)?, None, "{path}");
        }
        Ok(())
    }

    #[test]
    fn case_is_ignored_in_the_extension_and_nowhere_else() -> Outcome {
        assert_eq!(place_in(&INDEXED, "img/a.png.bak")?, None);
        Ok(())
    }

    #[test]
    fn a_directory_may_contain_members_when_it_leads_into_a_slot() -> Outcome {
        let matcher = SetMatcher::new(&INDEXED)?;
        assert!(matcher.may_contain_members("docs"));
        assert!(matcher.may_contain_members("docs/a.parts"));
        for dir in ["docs/a.parts/x", "notes", ".scratch"] {
            assert!(!matcher.may_contain_members(dir), "{dir}");
        }
        Ok(())
    }

    #[test]
    fn a_matcher_answers_its_declarations_shape_and_presentation() -> Outcome {
        let matcher = SetMatcher::new(&PHOTOGRAPHED)?;
        assert_eq!(matcher.root(), "index.md");
        let slots: Vec<&str> = matcher
            .shape()
            .member_slots
            .iter()
            .map(|slot| slot.pattern)
            .collect();
        assert_eq!(slots, ["img/*.jpg"]);
        assert_eq!(matcher.presentation(), Presentation::EntryPoint);
        Ok(())
    }
}
