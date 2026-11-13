//! The one home of the table that maps a local work item kind to the Shortcut
//! entity `create` makes, and to the story type a search filter matches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryType {
    Feature,
    Bug,
    Chore,
}

impl StoryType {
    /// The story type Shortcut itself names `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "feature" => Some(Self::Feature),
            "bug" => Some(Self::Bug),
            "chore" => Some(Self::Chore),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Bug => "bug",
            Self::Chore => "chore",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewEntity {
    Story(StoryType),
    Epic,
}

/// A local kind the table has no Shortcut entity for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKind(pub String);

impl std::fmt::Display for UnknownKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "kind {:?} has no Shortcut entity — use story, bug, task, spike \
             or epic",
            self.0
        )
    }
}

impl std::error::Error for UnknownKind {}

impl NewEntity {
    /// An empty kind makes a feature story, which is what Shortcut makes when
    /// given no type.
    ///
    /// # Errors
    ///
    /// [`UnknownKind`] for a kind outside the table.
    pub fn for_kind(kind: &str) -> Result<Self, UnknownKind> {
        match kind {
            "" => Ok(Self::Story(StoryType::Feature)),
            "epic" => Ok(Self::Epic),
            other => story_type_for_kind(other)
                .map(Self::Story)
                .ok_or_else(|| UnknownKind(other.to_owned())),
        }
    }

    #[must_use]
    pub const fn type_name(self) -> &'static str {
        match self {
            Self::Story(story_type) => story_type.as_str(),
            Self::Epic => "epic",
        }
    }
}

#[must_use]
pub fn story_type_for_kind(kind: &str) -> Option<StoryType> {
    match kind {
        "story" => Some(StoryType::Feature),
        "bug" => Some(StoryType::Bug),
        "task" | "spike" => Some(StoryType::Chore),
        _ => None,
    }
}
