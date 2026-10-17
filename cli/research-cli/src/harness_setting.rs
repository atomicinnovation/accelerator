//! How a harness setting read from the environment is parsed.

use std::num::NonZeroU32;

pub fn is_set(setting: &str) -> bool {
    !setting.trim_matches(is_javascript_whitespace).is_empty()
}

/// Read as the harness's JavaScript parser reads its own settings: trimmed
/// as `String.prototype.trim` trims, an optionally signed run of ASCII
/// digits, at least 1.
pub fn positive_whole_number(setting: &str) -> Option<NonZeroU32> {
    let trimmed = setting.trim_matches(is_javascript_whitespace);
    let digits = trimmed.strip_prefix('+').unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    NonZeroU32::new(digits.parse().unwrap_or(u32::MAX))
}

/// JavaScript's `trim` keeps U+0085, which Rust counts as whitespace, and
/// strips U+FEFF, which Rust does not.
const fn is_javascript_whitespace(character: char) -> bool {
    (character.is_whitespace() && character != '\u{85}')
        || character == '\u{FEFF}'
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::positive_whole_number;

    #[test]
    fn a_setting_is_parsed_as_the_harness_parses_it() {
        let zero_padded = format!("{}8", "0".repeat(40));
        for (setting, number) in [
            ("8", 8),
            ("007", 7),
            ("+8", 8),
            (" 8 ", 8),
            ("\t8\n", 8),
            ("\u{FEFF}8", 8),
            ("30", 30),
            ("1", 1),
            (zero_padded.as_str(), 8),
        ] {
            assert_eq!(
                positive_whole_number(setting),
                NonZeroU32::new(number),
                "setting {setting:?}"
            );
        }
    }

    #[test]
    fn a_setting_the_harness_ignores_is_none() {
        for setting in [
            "0", "-5", "-0", "-", "abc", "20.5", "1e2", "1,000", "٨",
            "\u{85}8", "", "  ", "+",
        ] {
            assert_eq!(
                positive_whole_number(setting),
                None,
                "setting {setting:?}"
            );
        }
    }

    #[test]
    fn a_setting_too_large_for_u32_saturates() {
        assert_eq!(positive_whole_number("99999999999"), Some(NonZeroU32::MAX));
    }
}
