//! Whether a produced artefact repeats a configured credential.
//!
//! The scan reports the *name* of the offending variable and never its value,
//! so a report is safe to print, log and commit.
//!
//! A credential is matched not only verbatim but in the encoded shapes a model
//! is likely to transcribe it into: base64 (standard and URL-safe, padded or
//! not) and maximal percent-encoding (unreserved set, either hex casing); any
//! of those reflowed across a line wrap; and, for a colon-free value, the
//! leading characters of any form long enough to survive head truncation. Case
//! folding, hex, partial percent-encoding, HTML/JSON escapes, nested encodings
//! and head truncation of a colon-bearing value are out of scope.

use base64::engine::general_purpose::STANDARD;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::engine::general_purpose::URL_SAFE;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use percent_encoding::percent_encode;
use percent_encoding::AsciiSet;
use percent_encoding::NON_ALPHANUMERIC;

const PERCENT_ESCAPE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

const MINIMUM_FORM_LENGTH: usize = 16;
const PREFIX_LENGTH: usize = 12;

/// A configured credential, paired with the variable that named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedSecret {
    pub name: String,
    pub value: String,
}

impl NamedSecret {
    /// The substrings whose presence in an artefact constitutes a leak.
    fn needles(&self) -> Vec<String> {
        let prefixable = self.head_prefixable();
        self.values()
            .iter()
            .flat_map(|value| needles_for(value, prefixable))
            .collect()
    }

    fn values(&self) -> Vec<String> {
        let mut values = vec![self.value.clone()];
        if let Some(half) = self.header_value_half() {
            values.push(half.to_owned());
        }
        values
    }

    /// `ACCELERATOR_BROWSER_AUTH_HEADER` holds a whole `Name: value` pair, and
    /// the daemon splits it on the first colon — so the value half is a needle
    /// of its own. Without it, an artefact rendering just the bearer token,
    /// the likely leakage shape, would match nothing.
    fn header_value_half(&self) -> Option<&str> {
        let (_, half) = self.value.split_once(':')?;
        let half = half.trim();
        (!half.is_empty()).then_some(half)
    }

    /// A colon marks a structural head (`AUTH_HEADER`, `LOGIN_URL`), whose
    /// leading characters recur in ordinary prose and unrelated encodings, so
    /// head-prefixing it would false-positive. Read from the whole value so the
    /// colon-free value-half inherits the same refusal.
    fn head_prefixable(&self) -> bool {
        !self.value.contains(':')
    }
}

fn needles_for(value: &str, prefixable: bool) -> Vec<String> {
    let mut needles = vec![value.to_owned()];
    if prefixable {
        needles.extend(head_prefix(value));
    }
    for encoding in encodings(value) {
        if prefixable {
            needles.extend(head_prefix(&encoding));
        }
        needles.push(encoding);
    }
    needles
}

fn head_prefix(form: &str) -> Option<String> {
    (form.chars().count() >= MINIMUM_FORM_LENGTH)
        .then(|| form.chars().take(PREFIX_LENGTH).collect())
}

fn encodings(value: &str) -> Vec<String> {
    let mut encodings: Vec<String> =
        [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
            .iter()
            .map(|engine| engine.encode(value))
            .collect();
    encodings.extend(percent_forms(value));
    encodings
}

fn percent_forms(value: &str) -> Vec<String> {
    let upper =
        percent_encode(value.as_bytes(), PERCENT_ESCAPE_SET).to_string();
    let lower = lowercase_percent_triplets(&upper);
    if lower == upper {
        vec![upper]
    } else {
        vec![upper, lower]
    }
}

fn lowercase_percent_triplets(encoded: &str) -> String {
    let mut lowered = String::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(character) = chars.next() {
        lowered.push(character);
        if character == '%' {
            let hex = chars.by_ref().take(2);
            lowered.extend(hex.map(|h| h.to_ascii_lowercase()));
        }
    }
    lowered
}

/// The names of every secret whose value appears in `body`, in the order the
/// secrets were given.
#[must_use]
pub fn scan(body: &str, secrets: &[NamedSecret]) -> Vec<String> {
    let stripped = strip_whitespace(body);
    secrets
        .iter()
        .filter(|secret| !secret.value.is_empty())
        .filter(|secret| {
            secret.needles().iter().any(|needle| {
                body.contains(needle.as_str())
                    || stripped.contains(needle.as_str())
            })
        })
        .map(|secret| secret.name.clone())
        .collect()
}

fn strip_whitespace(body: &str) -> String {
    body.chars().filter(|c| !c.is_ascii_whitespace()).collect()
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::scan;
    use super::NamedSecret;

    fn secret(name: &str, value: &str) -> NamedSecret {
        NamedSecret {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn a_clean_body_names_nothing() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "hunter2")];
        assert!(scan("a perfectly ordinary inventory", &secrets).is_empty());
    }

    #[test]
    fn a_verbatim_value_names_its_variable() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "hunter2")];
        assert_eq!(
            scan("the password is hunter2, oops", &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn an_empty_value_is_not_a_needle() {
        let secrets = [secret("ACCELERATOR_BROWSER_USERNAME", "")];
        assert!(scan("anything at all", &secrets).is_empty());
    }

    /// Matching only the whole `Name: value` pair would miss an artefact
    /// rendering just the token, which is the likely leakage shape.
    #[test]
    fn the_value_half_of_a_header_pair_is_a_needle_of_its_own() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_AUTH_HEADER",
            "Authorization: Bearer abc123",
        )];
        assert_eq!(
            scan("the request carried Bearer abc123", &secrets),
            vec!["ACCELERATOR_BROWSER_AUTH_HEADER"]
        );
    }

    #[test]
    fn the_header_name_alone_does_not_false_positive() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_AUTH_HEADER",
            "Authorization: Bearer abc123",
        )];
        assert!(scan(
            "the page sends an Authorization header on every request",
            &secrets
        )
        .is_empty());
    }

    #[test]
    fn the_report_never_carries_the_value() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "hunter2")];
        let report = scan("hunter2", &secrets).join(" ");
        assert!(!report.contains("hunter2"));
        assert!(report.contains("ACCELERATOR_BROWSER_PASSWORD"));
    }

    #[test]
    fn every_offending_variable_is_named_not_only_the_first() {
        let secrets = [
            secret("ACCELERATOR_BROWSER_USERNAME", "alice"),
            secret("ACCELERATOR_BROWSER_PASSWORD", "hunter2"),
        ];
        assert_eq!(
            scan("alice / hunter2", &secrets),
            vec![
                "ACCELERATOR_BROWSER_USERNAME",
                "ACCELERATOR_BROWSER_PASSWORD"
            ]
        );
    }

    #[test]
    fn a_base64_encoded_value_names_its_variable() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "s3cr3t-token")];
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("s3cr3t-token");
        assert_eq!(
            scan(&format!("the header read {encoded}"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_percent_encoded_value_names_its_variable() {
        let secrets = [secret("ACCELERATOR_BROWSER_LOGIN_URL", "a b/c")];
        assert_eq!(
            scan("the link was a%20b%2Fc", &secrets),
            vec!["ACCELERATOR_BROWSER_LOGIN_URL"]
        );
    }

    #[test]
    fn a_base64_encoded_header_value_half_names_its_variable() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_AUTH_HEADER",
            "Authorization: Bearer xyz",
        )];
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("Bearer xyz");
        assert_eq!(
            scan(&format!("the token was {encoded}"), &secrets),
            vec!["ACCELERATOR_BROWSER_AUTH_HEADER"]
        );
    }

    #[test]
    fn a_percent_encoded_header_value_half_names_its_variable() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_AUTH_HEADER",
            "Authorization: Bearer xyz",
        )];
        assert_eq!(
            scan("the token was Bearer%20xyz", &secrets),
            vec!["ACCELERATOR_BROWSER_AUTH_HEADER"]
        );
    }

    #[test]
    fn an_encoded_match_never_carries_the_value() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "s3cr3t-token")];
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("s3cr3t-token");
        let report = scan(&format!("leaked {encoded}"), &secrets).join(" ");
        assert!(!report.contains("s3cr3t-token"));
        assert!(!report.contains(&encoded));
        assert!(report.contains("ACCELERATOR_BROWSER_PASSWORD"));
    }

    #[test]
    fn a_url_safe_base64_encoded_value_names_its_variable() {
        let value = "???";
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        let standard = base64::engine::general_purpose::STANDARD.encode(value);
        let url_safe = base64::engine::general_purpose::URL_SAFE.encode(value);
        assert_ne!(url_safe, standard);
        assert_eq!(
            scan(&format!("the token read {url_safe}"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn an_unpadded_base64_encoded_value_names_its_variable() {
        let value = "hunter22";
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        let standard = base64::engine::general_purpose::STANDARD.encode(value);
        let no_pad =
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
        assert!(standard.ends_with('='));
        assert_ne!(no_pad, standard);
        assert_eq!(
            scan(&format!("the token read {no_pad}"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_lowercase_percent_encoded_value_names_its_variable() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "a/c")];
        let upper = "a%2Fc";
        let lower = "a%2fc";
        assert_ne!(lower, upper);
        assert_eq!(
            scan(&format!("the path was {lower}"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_value_whose_percent_form_needs_no_lowercasing_is_still_caught() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "a b")];
        assert_eq!(
            scan("the path was a%20b", &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_value_reflowed_across_a_line_wrap_names_its_variable() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "hunter2")];
        assert_eq!(
            scan("the token was hun\nter2 in the log", &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_reflowed_base64_form_is_still_caught() {
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", "s3cr3t-token")];
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("s3cr3t-token");
        let (head, tail) = encoded.split_at(8);
        assert_eq!(
            scan(&format!("body {head}\n{tail} end"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_reflowed_interior_whitespace_needle_is_not_reconstituted() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_AUTH_HEADER",
            "Authorization: Bearer abc123",
        )];
        assert!(scan("the token was Bearerabc123 here", &secrets).is_empty());
        let encoded =
            base64::engine::general_purpose::STANDARD.encode("Bearer abc123");
        let (head, tail) = encoded.split_at(6);
        assert_eq!(
            scan(&format!("the token was {head}\n{tail} here"), &secrets),
            vec!["ACCELERATOR_BROWSER_AUTH_HEADER"]
        );
    }

    #[test]
    fn a_raw_value_truncated_after_its_head_names_its_variable() {
        let value = "s3cr3t-p4ssw0rd-x";
        let head: String = value.chars().take(12).collect();
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert_eq!(
            scan(&format!("the token began {head} and was cut"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_truncated_base64_head_names_its_variable() {
        let value = "s3cr3t-token";
        let encoded = base64::engine::general_purpose::STANDARD.encode(value);
        let head: String = encoded.chars().take(12).collect();
        let body = format!("the header read {head} only");
        assert!(!body.contains(&encoded));
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert_eq!(scan(&body, &secrets), vec!["ACCELERATOR_BROWSER_PASSWORD"]);
    }

    #[test]
    fn a_truncated_percent_encoded_head_names_its_variable() {
        let value = "a/b/c/d/e/f";
        let percent = percent_encoding::percent_encode(
            value.as_bytes(),
            super::PERCENT_ESCAPE_SET,
        )
        .to_string();
        assert!(percent.chars().count() >= 16);
        let head: String = percent.chars().take(12).collect();
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert_eq!(
            scan(&format!("the path was {head} cut"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_short_value_gains_a_prefix_through_its_longer_base64() {
        let value = "hunter2-token";
        assert!(value.chars().count() < 16);
        let encoded = base64::engine::general_purpose::STANDARD.encode(value);
        assert!(encoded.chars().count() >= 16);
        let head: String = encoded.chars().take(12).collect();
        let body = format!("the header read {head} only");
        assert!(!body.contains(&encoded));
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert_eq!(scan(&body, &secrets), vec!["ACCELERATOR_BROWSER_PASSWORD"]);
    }

    #[test]
    fn a_multibyte_value_truncated_after_its_head_names_its_variable() {
        let value = "abcdefghijk€lmnop";
        assert!(value.chars().count() >= 16);
        let head: String = value.chars().take(12).collect();
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert_eq!(
            scan(&format!("the token was {head} cut"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_short_multibyte_value_gains_no_prefix() {
        let value = "€".repeat(13);
        assert!(value.len() >= 16 && value.chars().count() < 16);
        let head: String = value.chars().take(12).collect();
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", &value)];
        assert!(scan(&format!("the token was {head} cut"), &secrets).is_empty());
    }

    #[test]
    fn a_structured_value_head_is_not_flagged_in_any_form() {
        for (name, value) in [
            (
                "ACCELERATOR_BROWSER_AUTH_HEADER",
                "Authorization: Bearer secrettoken",
            ),
            (
                "ACCELERATOR_BROWSER_LOGIN_URL",
                "https://example.com/loginpath",
            ),
        ] {
            let secrets = [secret(name, value)];
            let raw_head: String = value.chars().take(12).collect();
            let base64_head: String = base64::engine::general_purpose::STANDARD
                .encode(value)
                .chars()
                .take(12)
                .collect();
            let percent_head: String = percent_encoding::percent_encode(
                value.as_bytes(),
                super::PERCENT_ESCAPE_SET,
            )
            .to_string()
            .chars()
            .take(12)
            .collect();
            for head in [&raw_head, &base64_head, &percent_head] {
                assert_eq!(head.chars().count(), 12);
                assert!(scan(
                    &format!("the value began {head} then stopped"),
                    &secrets
                )
                .is_empty());
            }
        }
    }

    #[test]
    fn a_colon_bearing_password_is_not_head_prefixed() {
        let value = "token:abcdefghijkl";
        assert!(value.chars().count() >= 16 && value.contains(':'));
        let head: String = value.chars().take(12).collect();
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        assert!(scan(&format!("the value began {head} then cut"), &secrets)
            .is_empty());
    }

    #[test]
    fn the_prefix_boundary_holds_at_exactly_twelve_characters() {
        let value = "abcdefghijklmnop";
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        let eleven: String = value.chars().take(11).collect();
        let twelve: String = value.chars().take(12).collect();
        assert!(scan(&format!("shares {eleven}Z only"), &secrets).is_empty());
        assert_eq!(
            scan(&format!("shares {twelve} only"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_form_shorter_than_sixteen_gains_no_prefix() {
        let value = "shortsecret12";
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        let head: String = value.chars().take(12).collect();
        assert!(scan(&format!("only {head} here"), &secrets).is_empty());
        assert_eq!(
            scan(&format!("all {value} here"), &secrets),
            vec!["ACCELERATOR_BROWSER_PASSWORD"]
        );
    }

    #[test]
    fn a_value_truncated_in_its_middle_or_tail_is_not_flagged() {
        let value = "abcdefghijklmnopqrst";
        let secrets = [secret("ACCELERATOR_BROWSER_PASSWORD", value)];
        let tail: String = value.chars().skip(8).collect();
        assert!(scan(&format!("only {tail} survived"), &secrets).is_empty());
    }

    #[test]
    fn a_high_entropy_but_legitimate_substring_is_not_flagged() {
        let secrets = [secret(
            "ACCELERATOR_BROWSER_PASSWORD",
            "correct-horse-battery-staple",
        )];
        assert!(
            scan("the trace id was 9f3a7b2c1d8e4f6a0b5c today", &secrets)
                .is_empty()
        );
    }

    #[test]
    fn head_prefix_gains_nothing_at_fifteen_and_a_head_at_sixteen() {
        assert_eq!(super::head_prefix("123456789012345"), None);
        assert_eq!(
            super::head_prefix("1234567890123456"),
            Some("123456789012".to_owned())
        );
    }
}
