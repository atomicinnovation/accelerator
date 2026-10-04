//! The filename-safe form of a question that a pair's stem carries.

const SLUG_MAX_LEN: usize = 60;
const EMPTY_SLUG: &str = "focus-area";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionSlug(String);

impl QuestionSlug {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for QuestionSlug {
    fn from(question: &str) -> Self {
        let lowered = question.to_ascii_lowercase();
        let hyphenated = lowered
            .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
            .filter(|run| !run.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let cut = hyphenated
            .get(..SLUG_MAX_LEN)
            .unwrap_or(&hyphenated)
            .trim_end_matches('-');
        Self(if cut.is_empty() {
            EMPTY_SLUG.to_owned()
        } else {
            cut.to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::QuestionSlug;

    #[test]
    fn a_question_slug_keeps_ascii_alphanumerics_and_hyphenates_the_rest() {
        assert_eq!(
            QuestionSlug::from("How do attention heads specialise?").as_str(),
            "how-do-attention-heads-specialise"
        );
        assert_eq!(QuestionSlug::from("  --Why?? ").as_str(), "why");
    }

    #[test]
    fn a_question_with_no_ascii_alphanumerics_slugs_to_focus_area() {
        assert_eq!(QuestionSlug::from("注意力机制？").as_str(), "focus-area");
        assert_eq!(QuestionSlug::from("").as_str(), "focus-area");
    }

    #[test]
    fn a_question_slug_is_cut_to_sixty_characters() {
        let sixty = "a".repeat(60);
        assert_eq!(QuestionSlug::from(sixty.as_str()).as_str(), sixty);
        let sixty_one = "b".repeat(61);
        assert_eq!(
            QuestionSlug::from(sixty_one.as_str()).as_str(),
            "b".repeat(60)
        );
    }

    #[test]
    fn a_cut_never_leaves_a_trailing_hyphen() {
        let question = format!("{} tail", "c".repeat(59));
        assert_eq!(
            QuestionSlug::from(question.as_str()).as_str(),
            "c".repeat(59)
        );
    }
}
