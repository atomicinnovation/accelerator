//! The provisional identity of a work item no tracker has confirmed yet.

const SUFFIX_LENGTH: usize = 6;

const fn is_crockford_character(c: char) -> bool {
    c.is_ascii_digit()
        || (c.is_ascii_lowercase() && !matches!(c, 'i' | 'l' | 'o' | 'u'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftId(String);

impl DraftId {
    pub const PREFIX: &'static str = "draft-";

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let canonical = raw.to_ascii_lowercase();
        let suffix = canonical.strip_prefix(Self::PREFIX)?;
        let well_formed = suffix.len() == SUFFIX_LENGTH
            && suffix.chars().all(is_crockford_character)
            && suffix.chars().any(|c| c.is_ascii_alphabetic());
        well_formed.then_some(Self(canonical))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::DraftId;

    #[test]
    fn a_draft_id_has_the_draft_prefix_and_six_crockford_characters() {
        assert_eq!(
            DraftId::parse("draft-k7mq3x").map(|id| id.as_str().to_owned()),
            Some("draft-k7mq3x".to_owned())
        );
        assert!(DraftId::parse("draft-k7mq3").is_none());
        assert!(DraftId::parse("draft-k7mq3xy").is_none());
        assert!(DraftId::parse("draft-k7mq3i").is_none());
        assert!(DraftId::parse("drafts-k7mq3x").is_none());
    }

    #[test]
    fn a_draft_id_is_read_case_insensitively_into_lowercase() {
        assert_eq!(
            DraftId::parse("DRAFT-K7MQ3X").map(|id| id.as_str().to_owned()),
            Some("draft-k7mq3x".to_owned())
        );
    }

    #[test]
    fn an_all_digit_draft_suffix_is_not_a_draft_id() {
        assert!(DraftId::parse("draft-123456").is_none());
    }
}
