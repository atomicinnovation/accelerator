//! The `metadata` command layer.
//!
//! Unlike the other three noun groups' command layers, this one is not made
//! generic over an injected port: `derive`/`render` are themselves already
//! generic over `corpus::metadata::Clock`, and this function's only
//! remaining decision — render on success, map the error on failure — is
//! already directly testable once given an already-resolved `Result` rather
//! than calling `derive_at` itself. The fallible, environment-dependent
//! construction (`SystemClock::try_new`, wrapped by `derive_at`) stays in
//! `main.rs`.

use corpus::ArtifactMetadata;
use corpus::FilenameTimestampFormat;
use corpus_adapters::metadata::render;
use corpus_adapters::ClockError;

use crate::outcome::Outcome;

/// Runs `metadata derive`.
///
/// # Errors
///
/// A [`kernel::Error`] carrying `metadata`'s `ClockError` text when the host
/// clock could not be resolved.
pub fn run_derive(
    metadata: Result<ArtifactMetadata, ClockError>,
    format: FilenameTimestampFormat,
) -> Result<Outcome, kernel::Error> {
    match metadata {
        Ok(metadata) => Ok(Outcome {
            stdout: render(&metadata, format),
            stderr: String::new(),
        }),
        Err(error) => Err(kernel::Error::Failed(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use corpus::ArtifactMetadata;
    use corpus::FilenameTimestampFormat;
    use corpus_adapters::ClockError;

    use super::run_derive;

    #[test]
    fn ok_renders_the_block() -> Result<(), Box<dyn std::error::Error>> {
        let metadata = ArtifactMetadata {
            datetime_utc: "2026-07-13T09:05:07+00:00".to_owned(),
            filename_timestamp: "2026-07-13_09-05-07".to_owned(),
            repository_name: Some("accelerator".to_owned()),
            revision: Some("abc123".to_owned()),
        };
        let outcome = run_derive(
            Ok(metadata),
            FilenameTimestampFormat::DateTimeUnderscored,
        )?;
        assert!(outcome
            .stdout
            .contains("Current Date/Time (UTC): 2026-07-13T09:05:07+00:00"));
        assert!(outcome.stdout.contains("Current Revision: abc123"));
        assert!(outcome.stdout.contains("Repository Name: accelerator"));
        assert_eq!(outcome.stderr, String::new());
        Ok(())
    }

    #[test]
    fn err_maps_to_a_failed_kernel_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = ClockError::new("no tzdata");
        let result = run_derive(
            Err(error),
            FilenameTimestampFormat::DateTimeUnderscored,
        );
        let Err(kernel::Error::Failed(message)) = result else {
            return Err("expected Err(kernel::Error::Failed(_))".into());
        };
        assert!(message.contains("no tzdata"));
        Ok(())
    }
}
