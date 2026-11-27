//! The set model: a doc type whose entries are directories, each holding a
//! root document and the members its declared shape names.

mod rename;
mod shape;

pub use crate::set::rename::pair_case_only_renames;
pub use crate::set::rename::CaseOnlyRename;
pub use crate::set::rename::MemberPath;

pub use crate::set::shape::SetMatcher;
pub use crate::set::shape::SetPlace;
pub use crate::set::shape::SlotPattern;
pub use crate::set::shape::SlotPatternError;

/// What a set type's owner declares about its sets: their shape and how they
/// are presented.
#[derive(Debug)]
pub struct SetDeclaration {
    pub shape: SetShape,
    pub presentation: Presentation,
}

/// The files a set directory holds: its root, named exactly, and the slots
/// its members fill, as patterns relative to the set directory.
#[derive(Debug)]
pub struct SetShape {
    pub root: &'static str,
    pub member_slots: &'static [MemberSlot],
}

#[derive(Debug)]
pub struct MemberSlot {
    pub pattern: &'static str,
    pub kind: MemberKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Document,
    Asset,
}

/// How a set is shown: as a page of its members, or as the one entry point
/// its root links onward from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    SetPage,
    EntryPoint,
}
