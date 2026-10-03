//! The Unicode data questions are folded and screened with.

use icu_normalizer::ComposingNormalizerBorrowed;
use icu_properties::props::DefaultIgnorableCodePoint;
use icu_properties::props::GeneralCategory;
use icu_properties::CodePointMapData;
use icu_properties::CodePointSetData;
use research::question::UnicodeText;

/// NFKC folding, and the general categories and default-ignorable code
/// points that render as nothing or as a line break.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnicodeTables;

impl UnicodeText for UnicodeTables {
    fn fold(&self, text: &str) -> String {
        ComposingNormalizerBorrowed::new_nfkc()
            .normalize(text)
            .into_owned()
    }

    fn is_hidden(&self, character: char) -> bool {
        matches!(
            CodePointMapData::<GeneralCategory>::new().get(character),
            GeneralCategory::Control
                | GeneralCategory::Format
                | GeneralCategory::PrivateUse
                | GeneralCategory::Unassigned
                | GeneralCategory::LineSeparator
                | GeneralCategory::ParagraphSeparator
        ) || CodePointSetData::new::<DefaultIgnorableCodePoint>()
            .contains(character)
    }
}

#[cfg(test)]
mod tests {
    use research::question::plain_question_breach;
    use research::question::PlainQuestionRule;
    use research::question::UnicodeText;

    use super::UnicodeTables;

    #[test]
    fn unicode_tables_hide_every_refused_class() {
        for hidden in [
            '\u{7}',
            '\u{202E}',
            '\u{E0041}',
            '\u{FE0F}',
            '\u{E0100}',
            '\u{034F}',
            '\u{3164}',
            '\u{2028}',
            '\u{E000}',
            '\u{0378}',
        ] {
            assert!(
                UnicodeTables.is_hidden(hidden),
                "{:04X}",
                u32::from(hidden)
            );
        }
        for shown in ['é', 'ï', 'Ж', 'я', 'a', ' ', '?', '注'] {
            assert!(!UnicodeTables.is_hidden(shown), "{shown}");
        }
    }

    #[test]
    fn unicode_tables_fold_fullwidth_punctuation() {
        assert_eq!(UnicodeTables.fold("\u{FF0E}"), ".");
        assert_eq!(UnicodeTables.fold("\u{FF0F}"), "/");
        assert_eq!(UnicodeTables.fold("\u{FF61}"), "\u{3002}");
    }

    #[test]
    fn unicode_tables_fold_a_combining_sequence_into_its_composed_form() {
        assert_eq!(UnicodeTables.fold("e\u{0301}"), "\u{E9}");
        assert_eq!(UnicodeTables.fold("\u{FB01}\u{0301}"), "f\u{ED}");
    }

    #[test]
    fn the_real_tables_refuse_a_halfwidth_dotted_host() {
        assert_eq!(
            plain_question_breach("see evil\u{FF61}example/q", &UnicodeTables),
            Some(PlainQuestionRule::Link)
        );
    }
}
