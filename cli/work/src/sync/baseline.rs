//! What a sync baseline records of each side, under either digest recipe.

/// The legacy recipe: one hash of the whole local file, one of the whole
/// projected remote body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyHashes {
    pub local: Option<String>,
    pub remote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFingerprints {
    pub frontmatter: String,
    pub fields: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFingerprints {
    pub body: String,
    pub fields: String,
}

/// The split recipe: the local frontmatter, synced fields and body, and the
/// remote body and synced fields, each fingerprinted on its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitFingerprints {
    pub local: Option<LocalFingerprints>,
    pub remote: Option<RemoteFingerprints>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineFingerprints {
    Legacy(LegacyHashes),
    Split(SplitFingerprints),
}

/// Both recipes' digests of the same pair of bodies, so a split entry stays
/// readable by a binary that knows only the legacy recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualRecipeFingerprints {
    pub split: SplitFingerprints,
    pub legacy: LegacyHashes,
}
