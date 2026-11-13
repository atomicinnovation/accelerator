//! A Shortcut entity's key as a work item carries it: `sc-<id>` for a story,
//! `epic-<id>` for an epic.
//!
//! The API takes the bare number, but a bare number cannot say which endpoint
//! it belongs to, and the port hands every call after `create` the key alone.
//! The prefix records that choice. `sc-` is Shortcut's own branch and commit
//! convention; `epic-` has no Shortcut counterpart.

use std::fmt;

const STORY_PREFIX: &str = "sc";
const EPIC_PREFIX: &str = "epic";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoryId(u64);

impl StoryId {
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EpicId(u64);

impl EpicId {
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    Story(StoryId),
    Epic(EpicId),
}

/// The v4 resource collection an entity lives under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Collection {
    Stories,
    Epics,
}

impl Collection {
    #[must_use]
    pub const fn path_segment(self) -> &'static str {
        match self {
            Self::Stories => "stories",
            Self::Epics => "epics",
        }
    }
}

/// An epic key where only a story will do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAStory(pub EpicId);

/// Text that is neither `sc-<id>` nor `epic-<id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnparseableKey(pub String);

impl fmt::Display for UnparseableKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} is not a Shortcut key — expected sc-<id> or epic-<id>",
            self.0
        )
    }
}

impl std::error::Error for UnparseableKey {}

impl ShortcutKey {
    /// Accepts either prefix in any case. A number with a leading zero is
    /// refused, so each entity has exactly one spelling.
    ///
    /// # Errors
    ///
    /// [`UnparseableKey`] for anything other than `sc-<id>` or `epic-<id>`.
    pub fn parse(text: &str) -> Result<Self, UnparseableKey> {
        let refused = || UnparseableKey(text.to_owned());
        let (prefix, digits) = text.split_once('-').ok_or_else(refused)?;
        let number = canonical_number(digits).ok_or_else(refused)?;
        if prefix.eq_ignore_ascii_case(STORY_PREFIX) {
            Ok(Self::Story(StoryId(number)))
        } else if prefix.eq_ignore_ascii_case(EPIC_PREFIX) {
            Ok(Self::Epic(EpicId(number)))
        } else {
            Err(refused())
        }
    }

    #[must_use]
    pub const fn collection(self) -> Collection {
        match self {
            Self::Story(_) => Collection::Stories,
            Self::Epic(_) => Collection::Epics,
        }
    }

    #[must_use]
    pub const fn number(self) -> u64 {
        match self {
            Self::Story(story) => story.0,
            Self::Epic(epic) => epic.0,
        }
    }

    /// # Errors
    ///
    /// [`NotAStory`] for an epic key.
    pub const fn into_story(self) -> Result<StoryId, NotAStory> {
        match self {
            Self::Story(story) => Ok(story),
            Self::Epic(epic) => Err(NotAStory(epic)),
        }
    }
}

fn canonical_number(digits: &str) -> Option<u64> {
    let well_formed = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.starts_with('0');
    well_formed.then(|| digits.parse().ok()).flatten()
}

impl fmt::Display for ShortcutKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Story(story) => {
                write!(formatter, "{STORY_PREFIX}-{}", story.0)
            }
            Self::Epic(epic) => write!(formatter, "{EPIC_PREFIX}-{}", epic.0),
        }
    }
}
