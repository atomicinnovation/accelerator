/// Where the library shows a doc type: its phase, and its position among
/// that phase's types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryPlacement {
    pub phase: LibraryPhase,
    pub position: u8,
}

/// A stage of the delivery lifecycle the library groups doc types under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryPhase {
    Define,
    Discover,
    Build,
    Ship,
    Operate,
    Remember,
}

/// A dimension the library filters a doc type's documents by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facet {
    Status,
    Project,
    ClusterSlug,
}

impl Facet {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Project => "project",
            Self::ClusterSlug => "clusterSlug",
        }
    }
}
