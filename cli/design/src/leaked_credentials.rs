//! Whether a produced artefact repeats a configured credential verbatim.
//!
//! The scan reports the *name* of the offending variable and never its value,
//! so a report is safe to print, log and commit.
//!
//! Matching is literal-substring only. The artefacts scanned are
//! model-authored prose, where a credential is as likely to appear
//! base64-encoded, percent-encoded, whitespace-normalised or truncated; those
//! encodings are not derived here.

/// A configured credential, paired with the variable that named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedSecret {
    pub name: String,
    pub value: String,
}

impl NamedSecret {
    /// The substrings whose presence in an artefact constitutes a leak.
    ///
    /// `ACCELERATOR_BROWSER_AUTH_HEADER` holds a whole `Name: value` pair, and
    /// the daemon splits it on the first colon — so the value half is a needle
    /// of its own. Without it, an artefact rendering just the bearer token,
    /// which is the likely leakage shape, would match nothing.
    fn needles(&self) -> Vec<&str> {
        let mut needles = vec![self.value.as_str()];
        if let Some((_, value)) = self.value.split_once(':') {
            let value = value.trim();
            if !value.is_empty() {
                needles.push(value);
            }
        }
        needles
    }
}

/// The names of every secret whose value appears in `body`, in the order the
/// secrets were given.
#[must_use]
pub fn scan(body: &str, secrets: &[NamedSecret]) -> Vec<String> {
    secrets
        .iter()
        .filter(|secret| !secret.value.is_empty())
        .filter(|secret| {
            secret.needles().iter().any(|needle| body.contains(needle))
        })
        .map(|secret| secret.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
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
}
