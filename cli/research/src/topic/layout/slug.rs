//! The filename-safe forms of a question: a pair's stem and a report's name.

use crate::topic::layout::REPORTS;

const SLUG_MAX_LEN: usize = 60;
const SHORTEST_WORD_CUT: usize = 41;
const EMPTY_QUESTION_SLUG: &str = "focus-area";
const EMPTY_REPORT_SLUG: &str = "report";

fn slug_of(question: &str) -> Option<String> {
    let hyphenated = question
        .to_ascii_lowercase()
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|run| !run.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let cut = cut_before_the_overflowing_word(&hyphenated);
    (!cut.is_empty()).then(|| cut.to_owned())
}

fn cut_before_the_overflowing_word(slug: &str) -> &str {
    if slug.len() <= SLUG_MAX_LEN {
        return slug;
    }
    let window = slug.get(..=SLUG_MAX_LEN).unwrap_or(slug);
    match window.rfind('-') {
        Some(hyphen) if hyphen >= SHORTEST_WORD_CUT => &window[..hyphen],
        _ => slug
            .get(..SLUG_MAX_LEN)
            .unwrap_or(slug)
            .trim_end_matches('-'),
    }
}

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
        Self(
            slug_of(question).unwrap_or_else(|| EMPTY_QUESTION_SLUG.to_owned()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSlug(String);

impl ReportSlug {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Compares names without regard to ASCII case: a case-insensitive
    /// filesystem holds `A.MD` and `a.md` as one file.
    #[must_use]
    pub fn free_report_path(&self, taken: &[String]) -> String {
        let is_taken = |name: &str| {
            taken.iter().any(|entry| entry.eq_ignore_ascii_case(name))
        };
        let mut name = format!("{}.md", self.0);
        let mut suffix = 1_u32;
        while is_taken(&name) {
            suffix += 1;
            name = format!("{}-{suffix}.md", self.0);
        }
        format!("{REPORTS}/{name}")
    }
}

impl From<&str> for ReportSlug {
    fn from(question: &str) -> Self {
        Self(slug_of(question).unwrap_or_else(|| EMPTY_REPORT_SLUG.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::QuestionSlug;
    use super::ReportSlug;

    const C7: &str = "How do autonomous language model agents decompose long \
                      horizon planning tasks into executable steps?";
    const C8: &str =
        "Supercalifragilisticexpialidociousplannersoutperformallbaselines?";

    fn taken(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

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

    #[test]
    fn a_long_question_slug_is_cut_before_the_word_that_would_pass_sixty() {
        assert_eq!(
            QuestionSlug::from(C7).as_str(),
            "how-do-autonomous-language-model-agents-decompose-long"
        );
    }

    #[test]
    fn a_word_cut_leaving_forty_characters_or_fewer_is_a_cut_at_sixty() {
        let question = format!("{} {}", "a".repeat(40), "b".repeat(30));
        let hard_cut = format!("{}-{}", "a".repeat(40), "b".repeat(19));
        assert_eq!(QuestionSlug::from(question.as_str()).as_str(), hard_cut);
        assert_eq!(ReportSlug::from(question.as_str()).as_str(), hard_cut);
    }

    #[test]
    fn a_word_cut_leaving_forty_one_characters_is_kept() {
        let question = format!("{} {}", "a".repeat(41), "b".repeat(30));
        assert_eq!(
            QuestionSlug::from(question.as_str()).as_str(),
            "a".repeat(41)
        );
        assert_eq!(
            ReportSlug::from(question.as_str()).as_str(),
            "a".repeat(41)
        );
    }

    #[test]
    fn a_short_opening_before_an_overlong_word_is_cut_at_sixty() {
        let question = format!("How do {C8}");
        assert_eq!(
            ReportSlug::from(question.as_str()).as_str(),
            "how-do-supercalifragilisticexpialidociousplannersoutperforma"
        );
    }

    #[test]
    fn a_report_slug_keeps_ascii_alphanumerics_and_hyphenates_the_rest() {
        assert_eq!(
            ReportSlug::from("How do LLM agents plan multi-step tasks?")
                .as_str(),
            "how-do-llm-agents-plan-multi-step-tasks"
        );
    }

    #[test]
    fn a_sixty_character_report_slug_is_kept_whole() {
        let question = format!("b {}", "a".repeat(58));
        assert_eq!(
            ReportSlug::from(question.as_str()).as_str(),
            format!("b-{}", "a".repeat(58))
        );
    }

    #[test]
    fn a_long_report_slug_is_cut_before_its_last_hyphen_within_sixty_one_characters(
    ) {
        assert_eq!(
            ReportSlug::from(C7).as_str(),
            "how-do-autonomous-language-model-agents-decompose-long"
        );
    }

    #[test]
    fn a_long_report_slug_without_a_hyphen_is_cut_at_sixty_characters() {
        assert_eq!(
            ReportSlug::from(C8).as_str(),
            "supercalifragilisticexpialidociousplannersoutperformallbasel"
        );
    }

    #[test]
    fn a_report_slug_whose_sixty_first_character_is_a_hyphen_keeps_sixty() {
        let question = format!("b {} c", "a".repeat(58));
        assert_eq!(
            ReportSlug::from(question.as_str()).as_str(),
            format!("b-{}", "a".repeat(58))
        );
    }

    #[test]
    fn a_question_with_no_ascii_alphanumerics_slugs_to_report() {
        assert_eq!(ReportSlug::from("注意力？").as_str(), "report");
        assert_eq!(ReportSlug::from("").as_str(), "report");
    }

    #[test]
    fn an_untaken_report_path_is_the_bare_slug() {
        assert_eq!(ReportSlug::from("A").free_report_path(&[]), "reports/a.md");
    }

    #[test]
    fn a_taken_report_path_takes_the_first_free_suffix() {
        assert_eq!(
            ReportSlug::from("A").free_report_path(&taken(&["a.md", "a-2.md"])),
            "reports/a-3.md"
        );
    }

    #[test]
    fn a_report_name_taken_in_another_case_is_taken() {
        assert_eq!(
            ReportSlug::from("A").free_report_path(&taken(&["A.MD"])),
            "reports/a-2.md"
        );
    }

    #[test]
    fn a_suffixed_report_path_may_pass_sixty_characters() {
        let slug = ReportSlug::from(C8);
        let bare = format!("{}.md", slug.as_str());
        assert_eq!(
            slug.free_report_path(&taken(&[&bare])),
            "reports/supercalifragilisticexpialidociousplannersoutperformallbasel-2.md"
        );
    }
}
