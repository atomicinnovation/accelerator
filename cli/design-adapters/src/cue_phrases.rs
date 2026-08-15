//! The compiled cue-phrase matcher behind `design`'s port.

use design::cue_phrase_audit::CASE_SENSITIVE_CUE_PHRASE_PATTERN;
use design::CuePhraseMatcher;
use design::CUE_PHRASE_PATTERNS;
use regex::Regex;
use regex::RegexBuilder;

/// The domain's pattern slice, compiled.
///
/// Two expressions, not one: the case-sensitive alternative distinguishes
/// `implement Foo` from `implement foo`, so it cannot share a
/// case-insensitive expression with the rest.
pub struct CompiledCuePhrases {
    case_insensitive: Regex,
    case_sensitive: Regex,
}

impl CompiledCuePhrases {
    /// Compiles the canonical patterns.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error::Failed`] when a pattern does not compile, which can
    /// only happen if the domain's `const` slice is edited into an invalid
    /// ERE.
    pub fn new() -> Result<Self, kernel::Error> {
        let joined = CUE_PHRASE_PATTERNS.join("|");
        let compile = |pattern: &str, case_insensitive: bool| {
            RegexBuilder::new(pattern)
                .case_insensitive(case_insensitive)
                .build()
                .map_err(|error| {
                    kernel::Error::Failed(format!(
                        "could not compile the cue-phrase pattern \
                         {pattern:?}: {error}"
                    ))
                })
        };
        Ok(Self {
            case_insensitive: compile(&joined, true)?,
            case_sensitive: compile(CASE_SENSITIVE_CUE_PHRASE_PATTERN, false)?,
        })
    }
}

impl CuePhraseMatcher for CompiledCuePhrases {
    fn matches(&self, text: &str) -> bool {
        self.case_insensitive.is_match(text)
            || self.case_sensitive.is_match(text)
    }
}

#[cfg(test)]
mod tests {
    use design::CuePhraseMatcher as _;

    use super::CompiledCuePhrases;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn each_case_insensitive_alternative_matches_in_either_case(
    ) -> Result<(), TestError> {
        let matcher = CompiledCuePhrases::new()?;
        for text in [
            "We need a thing",
            "we need a thing",
            "Users need a thing",
            "A user needs — no: user need",
            "The system must do it",
            "the system must do it",
        ] {
            assert!(matcher.matches(text), "{text}");
        }
        Ok(())
    }

    /// The one pattern whose case carries meaning: a proper-noun feature name
    /// matches, a lowercase verb phrase does not.
    #[test]
    fn the_implement_pattern_stays_case_sensitive_on_its_second_word(
    ) -> Result<(), TestError> {
        let matcher = CompiledCuePhrases::new()?;
        assert!(matcher.matches("Implement Dashboard"));
        assert!(matcher.matches("implement Dashboard"));
        assert!(!matcher.matches("implement dashboard"));
        Ok(())
    }

    #[test]
    fn prose_carrying_no_cue_phrase_does_not_match() -> Result<(), TestError> {
        let matcher = CompiledCuePhrases::new()?;
        assert!(!matcher.matches("This section describes the colours used."));
        Ok(())
    }
}
