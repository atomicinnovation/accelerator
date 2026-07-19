//! The atomic-store record model the JSONL composer renders.

/// The three-value outcome of an interactive transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Accepted,
    Edited,
    Skipped,
}

impl Outcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Edited => "edited",
            Self::Skipped => "skipped",
        }
    }
}

/// A single atomic-store record. `proposed_value` is required (emptiness is
/// rejected by the composer); `user_value` is presence-based; `extras` carry
/// author-declared fields in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub transformation_key: String,
    pub schema_version: u32,
    pub outcome: Outcome,
    pub proposed_value: String,
    pub user_value: Option<String>,
    pub timestamp: String,
    pub extras: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::Outcome;

    #[test]
    fn outcome_renders_its_canonical_token() {
        assert_eq!(Outcome::Accepted.as_str(), "accepted");
        assert_eq!(Outcome::Edited.as_str(), "edited");
        assert_eq!(Outcome::Skipped.as_str(), "skipped");
    }
}
