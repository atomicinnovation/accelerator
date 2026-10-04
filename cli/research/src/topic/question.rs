//! Questions as `conduct` compares and admits them: equal after Unicode
//! compatibility folding and whitespace collapsing, and plain when they carry
//! nothing a reader could follow as a link.

pub const LONGEST_PLAIN_QUESTION: usize = 300;

/// The Unicode data a question is folded and screened with, which a pure
/// domain cannot carry itself.
pub trait UnicodeText {
    /// The text under compatibility normalisation (NFKC).
    fn fold(&self, text: &str) -> String;

    /// Whether `character` renders as nothing or as a line break: a control,
    /// format, private-use, unassigned or separator code point, or a default
    /// ignorable one.
    fn is_hidden(&self, character: char) -> bool;
}

/// A question reduced to the form two questions are compared in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalisedQuestion(String);

impl NormalisedQuestion {
    #[must_use]
    pub fn of(question: &str, unicode: &dyn UnicodeText) -> Self {
        Self(
            unicode
                .fold(question)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlainQuestionRule {
    TooLong,
    HiddenCharacter,
    Link,
}

/// The first rule `text` breaks, if any.
///
/// Length counts the characters as written, before folding; the link rules
/// read the folded text, so a fullwidth or halfwidth form cannot disguise a
/// host.
pub fn plain_question_breach(
    text: &str,
    unicode: &dyn UnicodeText,
) -> Option<PlainQuestionRule> {
    if text.chars().count() > LONGEST_PLAIN_QUESTION {
        Some(PlainQuestionRule::TooLong)
    } else if text.chars().any(|character| unicode.is_hidden(character)) {
        Some(PlainQuestionRule::HiddenCharacter)
    } else if reads_as_a_link(&link_view(text, unicode)) {
        Some(PlainQuestionRule::Link)
    } else {
        None
    }
}

const IDEOGRAPHIC_FULL_STOP: char = '\u{3002}';
const LONGEST_LABEL: usize = 63;
const TOP_LEVEL_LABEL: std::ops::RangeInclusive<usize> = 2..=24;
const IPV4_LABELS: usize = 4;
const QUESTION_CLOSERS: [char; 6] =
    [')', ']', '"', '\'', '\u{201D}', '\u{2019}'];

// NFKC leaves U+3002 alone, but URL parsers split host labels on it.
fn link_view(text: &str, unicode: &dyn UnicodeText) -> Vec<char> {
    unicode
        .fold(text)
        .chars()
        .map(|c| if c == IDEOGRAPHIC_FULL_STOP { '.' } else { c })
        .collect()
}

fn reads_as_a_link(text: &[char]) -> bool {
    let joined: String = text.iter().collect();
    joined.contains("//")
        || has_www_token(&joined)
        || has_percent_escape(text)
        || (0..text.len())
            .any(|at| is_host_marker(text, at) && ends_a_host_name(&text[..at]))
}

fn has_www_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        token
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
            .starts_with("www.")
    })
}

fn has_percent_escape(text: &[char]) -> bool {
    text.windows(3).any(|window| {
        window[0] == '%'
            && window[1].is_ascii_hexdigit()
            && window[2].is_ascii_hexdigit()
    })
}

fn is_host_marker(text: &[char], at: usize) -> bool {
    let next = text.get(at + 1).copied();
    match text[at] {
        '/' | '\\' => true,
        ':' => next.is_some_and(|c| c.is_ascii_digit()),
        '?' => next.is_some_and(|c| {
            !c.is_whitespace() && !QUESTION_CLOSERS.contains(&c)
        }),
        _ => false,
    }
}

fn ends_a_host_name(before: &[char]) -> bool {
    let before = before.strip_suffix(&['.']).unwrap_or(before);
    let labels = trailing_labels(before);
    labels.len() >= 2 && (has_a_top_level_label(&labels) || is_ipv4(&labels))
}

/// The dotted labels that end `text`, rightmost first.
fn trailing_labels(text: &[char]) -> Vec<&[char]> {
    let mut labels = Vec::new();
    let mut end = text.len();
    loop {
        let start = text[..end]
            .iter()
            .rposition(|&c| !is_label_character(c))
            .map_or(0, |at| at + 1);
        let label = &text[start..end];
        if label.is_empty() || label.len() > LONGEST_LABEL {
            break;
        }
        labels.push(label);
        if start == 0 || text[start - 1] != '.' {
            break;
        }
        end = start - 1;
    }
    labels
}

fn is_label_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-'
}

fn has_a_top_level_label(labels: &[&[char]]) -> bool {
    let top = labels[0];
    TOP_LEVEL_LABEL.contains(&top.len())
        && top.iter().any(|c| c.is_alphabetic())
}

fn is_ipv4(labels: &[&[char]]) -> bool {
    labels.len() >= IPV4_LABELS
        && labels[..IPV4_LABELS]
            .iter()
            .all(|label| label.iter().all(char::is_ascii_digit))
}

/// Folds as NFKC does for the forms the tests use, and hides the code points
/// the tests name.
#[cfg(test)]
pub struct FakeUnicode;

#[cfg(test)]
impl UnicodeText for FakeUnicode {
    fn fold(&self, text: &str) -> String {
        text.chars()
            .map(|c| match c {
                '\u{FF0E}' => '.',
                '\u{FF0F}' => '/',
                '\u{FF1F}' => '?',
                '\u{FF61}' => IDEOGRAPHIC_FULL_STOP,
                other => other,
            })
            .collect()
    }

    fn is_hidden(&self, character: char) -> bool {
        character.is_control()
            || [
                '\u{202E}',
                '\u{E0041}',
                '\u{FE0F}',
                '\u{E0100}',
                '\u{034F}',
                '\u{3164}',
                '\u{2028}',
                '\u{E000}',
                '\u{0378}',
            ]
            .contains(&character)
    }
}

#[cfg(test)]
mod tests {
    use super::plain_question_breach;
    use super::FakeUnicode;
    use super::NormalisedQuestion;
    use super::PlainQuestionRule;
    use super::PlainQuestionRule::HiddenCharacter;
    use super::PlainQuestionRule::Link;
    use super::PlainQuestionRule::TooLong;

    fn breach(text: &str) -> Option<PlainQuestionRule> {
        plain_question_breach(text, &FakeUnicode)
    }

    #[test]
    fn questions_equal_after_folding_and_collapsing_whitespace() {
        let of = |text| NormalisedQuestion::of(text, &FakeUnicode);
        assert_eq!(of("  A \t b？"), of("A b?"));
        assert_eq!(of("A b?").as_str(), "A b?");
        assert_ne!(of("A b?"), of("A c?"));
    }

    #[test]
    fn every_plain_question_rule_gets_its_verdict() {
        let rows: [(&str, Option<PlainQuestionRule>); 34] = [
            ("line one\nline two?", Some(HiddenCharacter)),
            ("bell \u{7}?", Some(HiddenCharacter)),
            ("bidi \u{202E}?", Some(HiddenCharacter)),
            ("tag \u{E0041}?", Some(HiddenCharacter)),
            ("selector \u{FE0F}?", Some(HiddenCharacter)),
            ("selector \u{E0100}?", Some(HiddenCharacter)),
            ("filler \u{3164}?", Some(HiddenCharacter)),
            ("separator \u{2028}?", Some(HiddenCharacter)),
            ("private \u{E000}?", Some(HiddenCharacter)),
            ("unassigned \u{0378}?", Some(HiddenCharacter)),
            ("see evil\u{FF0E}example\u{FF0F}q", Some(Link)),
            ("see evil\u{3002}example/q", Some(Link)),
            ("see evil\u{FF61}example/q", Some(Link)),
            ("see evil.example:8080/q", Some(Link)),
            ("see evil.example\\q", Some(Link)),
            ("see //evil.example", Some(Link)),
            ("How does Node.js/Deno compare?", Some(Link)),
            ("What changed in Python 3.12?", None),
            ("How does it compare with Vue.js?", None),
            ("see evil.example?q=1", Some(Link)),
            (
                "see \u{43F}\u{440}\u{438}.\u{440}\u{444}/payload",
                Some(Link),
            ),
            ("Is café naïve?", None),
            ("see https://x.example", Some(Link)),
            ("see www.x.example", Some(Link)),
            ("see evil.example/q", Some(Link)),
            ("How is Node.js tuned?", None),
            ("see evil.example?/x", Some(Link)),
            ("see evil.example?%41", Some(Link)),
            ("see evil%2Eexample/q", Some(Link)),
            ("Is a 1.85:1 aspect ratio better?", None),
            ("Why is a 3.5/5 rating typical?", None),
            ("see 192.168.0.1/x", Some(Link)),
            ("在这种背景下。A/B测试如何设计？", None),
            ("Vue.js和React有何不同？各自适合什么场景？", Some(Link)),
        ];
        for (text, expected) in rows {
            assert_eq!(breach(text), expected, "{text:?}");
        }
    }

    #[test]
    fn a_question_ending_before_closing_punctuation_is_not_a_host() {
        for text in [
            "(Is it Vue.js?)",
            "Is it \"Vue.js?\"",
            "Is it ‘Vue.js?’",
            "Vue.js? Or React?",
        ] {
            assert_eq!(breach(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_trailing_dot_does_not_hide_a_host() {
        assert_eq!(breach("see evil.example./q"), Some(Link));
    }

    #[test]
    fn length_counts_characters_as_written() {
        assert_eq!(breach(&"a".repeat(300)), None);
        assert_eq!(breach(&"a".repeat(301)), Some(TooLong));
        assert_eq!(breach(&"é".repeat(300)), None);
        assert_eq!(breach(&"\u{FB01}".repeat(300)), None);
    }
}
