//! The command layer: compose a port, call the domain, render a [`Report`].
//!
//! The accept/reject split follows one rule. A **usage error** is a malformed
//! *invocation* — an unknown flag, a missing argument, an argument the tool
//! cannot interpret at all — and is carried by `kernel::Error::Refusal` to
//! exit 2. Anything the tool successfully evaluated and then rejected is a
//! **verdict**, and exits 1.
//!
//! That rule settles the cases an example list leaves open: `scrub-secrets` on
//! a nonexistent file is exit 2, because the argument cannot be interpreted as
//! a file to scan; `validate-source` on a path that exists but is not a
//! directory is exit 1, because it was evaluated and rejected.

use std::fmt::Write as _;
use std::path::Path;

use design::access_policy;
use design::credentials;
use design::cue_phrase_audit;
use design::leaked_credentials;
use design::source_location;
use design::Allowances;
use design::CuePhraseMatcher;
use design::DowngradeReason;
use design::SourceLocation;
use design::Verdict;
use design_adapters::filesystem;
use design_adapters::filesystem::DirectoryCheck;

use crate::report::Report;

/// Runs `validate-source`. Every outcome is a verdict, so nothing here can
/// fail.
#[must_use]
pub fn validate_source(
    location: &str,
    allowances: Allowances,
    directory_check: &dyn Fn(&Path) -> DirectoryCheck,
) -> Report {
    let parsed = match source_location::parse(location) {
        Ok(parsed) => parsed,
        Err(error) => return Report::rejected(&error.to_string()),
    };

    if let SourceLocation::RepositoryPath(path) = &parsed {
        if directory_check(Path::new(path)) == DirectoryCheck::NotADirectory {
            return Report::rejected(&format!(
                "location '{path}' does not exist or is not a directory."
            ));
        }
    }

    match access_policy::evaluate(&parsed, allowances) {
        Verdict::Accepted => Report::silent(),
        Verdict::Rejected(reason) => Report::rejected(&reason),
    }
}

/// Runs `resolve-auth`.
///
/// # Errors
///
/// A [`kernel::Error::Refusal`] when the form-login trio is partially
/// configured: the environment names an intent the tool cannot act on, which
/// is a malformed invocation rather than a judged input.
pub fn resolve_auth(
    credentials: &credentials::Credentials,
) -> Result<Report, kernel::Error> {
    let resolution = credentials::resolve(credentials)
        .map_err(|error| kernel::Error::Refusal(error.to_string()))?;
    let report = Report::line(&resolution.mode.to_string());
    Ok(match &resolution.warning {
        Some(warning) => report.warning(warning),
        None => report,
    })
}

/// Runs `scrub-secrets`.
///
/// # Errors
///
/// A [`kernel::Error::Refusal`] when the path names no readable file.
pub fn scrub_secrets(
    file: &Path,
    secrets: &[leaked_credentials::NamedSecret],
) -> Result<Report, kernel::Error> {
    let body = filesystem::read_document(file)?;
    let leaked = leaked_credentials::scan(&body, secrets);
    let Some(name) = leaked.first() else {
        return Ok(Report::silent());
    };
    Ok(Report::rejected(&format!(
        "the literal value of {name} appears in the generated inventory body. \
         The artifact was not written. Check your content for accidental \
         secret leakage."
    )))
}

/// Runs `notify-downgrade`. An unknown reason cannot reach here: clap rejects
/// it at parse time with its own usage exit.
#[must_use]
pub fn notify_downgrade(reason: DowngradeReason) -> Report {
    Report::line(reason.message())
}

/// Runs `audit-cue-phrases`.
///
/// # Errors
///
/// A [`kernel::Error::Refusal`] when the path names no readable file, or a
/// [`kernel::Error::Failed`] when the canonical patterns do not compile.
pub fn audit_cue_phrases(
    file: &Path,
    matcher: &dyn CuePhraseMatcher,
) -> Result<Report, kernel::Error> {
    let body = filesystem::read_document(file)?;
    let uncued = cue_phrase_audit::audit(&body, matcher);
    if uncued.is_empty() {
        return Ok(Report::silent());
    }
    let mut stderr = String::new();
    for section in uncued {
        let _ = writeln!(
            stderr,
            "error: H2 section '{}' has no cue-phrase paragraph. Add prose \
             matching one of: we need to / users need / the system must / \
             implement <ProperNoun>.",
            section.name
        );
    }
    Ok(Report::Rejected { stderr })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use design::credentials::Credentials;
    use design::leaked_credentials::NamedSecret;
    use design::Allowances;
    use design::CuePhraseMatcher;
    use design::DowngradeReason;
    use design_adapters::filesystem::DirectoryCheck;

    use super::audit_cue_phrases;
    use super::notify_downgrade;
    use super::resolve_auth;
    use super::scrub_secrets;
    use super::validate_source;
    use crate::report::Report;

    type TestError = Box<dyn std::error::Error>;

    fn always_a_directory(_path: &Path) -> DirectoryCheck {
        DirectoryCheck::Directory
    }

    fn never_a_directory(_path: &Path) -> DirectoryCheck {
        DirectoryCheck::NotADirectory
    }

    fn validate(location: &str) -> Report {
        validate_source(location, Allowances::default(), &always_a_directory)
    }

    fn is_rejected(report: &Report) -> bool {
        matches!(report, Report::Rejected { .. })
    }

    #[test]
    fn https_to_a_public_host_is_accepted_silently() {
        assert_eq!(validate("https://example.com"), Report::silent());
    }

    #[test]
    fn http_to_a_public_host_is_rejected_and_the_flag_recovers_it() {
        assert!(is_rejected(&validate("http://example.com")));
        assert_eq!(
            validate_source(
                "http://example.com",
                Allowances {
                    internal: false,
                    insecure_scheme: true,
                },
                &always_a_directory,
            ),
            Report::silent()
        );
    }

    /// Evaluated and rejected, so exit 1 — not a malformed invocation.
    #[test]
    fn a_path_that_is_not_a_directory_is_a_verdict_not_a_refusal(
    ) -> Result<(), TestError> {
        let report = validate_source(
            "./nowhere",
            Allowances::default(),
            &never_a_directory,
        );
        let Report::Rejected { stderr } = report else {
            return Err("expected a rejection".into());
        };
        assert!(stderr.contains("does not exist or is not a directory"));
        Ok(())
    }

    #[test]
    fn a_numeric_encoding_is_rejected_and_no_flag_recovers_it(
    ) -> Result<(), TestError> {
        for allowances in [
            Allowances::default(),
            Allowances {
                internal: true,
                insecure_scheme: true,
            },
        ] {
            let report = validate_source(
                "https://0x7f000001",
                allowances,
                &always_a_directory,
            );
            let Report::Rejected { stderr } = report else {
                return Err("expected a rejection".into());
            };
            assert!(stderr.contains("numeric IPv4 encoding"));
        }
        Ok(())
    }

    #[test]
    fn resolve_auth_prints_the_mode() -> Result<(), TestError> {
        assert_eq!(
            resolve_auth(&Credentials::default())?,
            Report::line("none")
        );
        Ok(())
    }

    #[test]
    fn a_partial_form_configuration_refuses_rather_than_rejecting(
    ) -> Result<(), TestError> {
        let credentials = Credentials {
            username: Some("alice".to_owned()),
            ..Credentials::default()
        };
        let Err(error) = resolve_auth(&credentials) else {
            return Err("expected a refusal".into());
        };
        assert!(matches!(error, kernel::Error::Refusal(_)));
        Ok(())
    }

    #[test]
    fn a_clean_artefact_scrubs_silently() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let file = work.path().join("inventory.md");
        fs::write(&file, "nothing to see")?;
        let secrets = [NamedSecret {
            name: "ACCELERATOR_BROWSER_PASSWORD".to_owned(),
            value: "hunter2".to_owned(),
        }];
        assert_eq!(scrub_secrets(&file, &secrets)?, Report::silent());
        Ok(())
    }

    #[test]
    fn a_leaked_value_names_its_variable_and_never_the_value(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let file = work.path().join("inventory.md");
        fs::write(&file, "the password is hunter2")?;
        let secrets = [NamedSecret {
            name: "ACCELERATOR_BROWSER_PASSWORD".to_owned(),
            value: "hunter2".to_owned(),
        }];
        let Report::Rejected { stderr } = scrub_secrets(&file, &secrets)?
        else {
            return Err("expected a rejection".into());
        };
        assert!(stderr.contains("ACCELERATOR_BROWSER_PASSWORD"));
        assert!(!stderr.contains("hunter2"));
        Ok(())
    }

    /// The argument cannot be interpreted as a file to scan, so it is a usage
    /// error — a deliberate split from the shell's conflated exit 1.
    #[test]
    fn scrubbing_a_nonexistent_file_refuses_rather_than_rejecting(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let Err(error) = scrub_secrets(&work.path().join("absent.md"), &[])
        else {
            return Err("expected a refusal".into());
        };
        assert!(matches!(error, kernel::Error::Refusal(_)));
        Ok(())
    }

    #[test]
    fn every_downgrade_reason_prints_its_own_message() {
        for reason in DowngradeReason::ALL {
            assert_eq!(
                notify_downgrade(reason),
                Report::line(reason.message())
            );
        }
    }

    struct NeverMatches;

    impl CuePhraseMatcher for NeverMatches {
        fn matches(&self, _text: &str) -> bool {
            false
        }
    }

    struct AlwaysMatches;

    impl CuePhraseMatcher for AlwaysMatches {
        fn matches(&self, _text: &str) -> bool {
            true
        }
    }

    #[test]
    fn a_fully_cued_document_audits_silently() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let file = work.path().join("gaps.md");
        fs::write(&file, "## Alpha\n\nprose\n")?;
        assert_eq!(audit_cue_phrases(&file, &AlwaysMatches)?, Report::silent());
        Ok(())
    }

    #[test]
    fn every_uncued_section_is_named_in_the_rejection() -> Result<(), TestError>
    {
        let work = tempfile::tempdir()?;
        let file = work.path().join("gaps.md");
        fs::write(&file, "## Alpha\n\nprose\n\n## Beta\n\nmore\n")?;
        let Report::Rejected { stderr } =
            audit_cue_phrases(&file, &NeverMatches)?
        else {
            return Err("expected a rejection".into());
        };
        assert!(stderr.contains("'Alpha'"));
        assert!(stderr.contains("'Beta'"));
        Ok(())
    }

    #[test]
    fn auditing_a_nonexistent_file_refuses_rather_than_rejecting(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let Err(error) =
            audit_cue_phrases(&work.path().join("absent.md"), &NeverMatches)
        else {
            return Err("expected a refusal".into());
        };
        assert!(matches!(error, kernel::Error::Refusal(_)));
        Ok(())
    }
}
