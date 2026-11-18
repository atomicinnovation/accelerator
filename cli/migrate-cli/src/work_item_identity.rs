//! The work-item identity migrations read, answered by the work context.

use migrate::ports::MigrationError;
use migrate::ports::WorkItemIdentity;
use work_adapters::config::scheme_from_settings_under_any_tracker;
use work_adapters::config::KeySource;

pub struct WorkIdentity;

fn failure(error: &impl ToString) -> MigrationError {
    MigrationError::new(error.to_string())
}

impl WorkItemIdentity for WorkIdentity {
    fn canonicalise(
        &self,
        raw: &str,
        pattern: &str,
        key: Option<&str>,
    ) -> Result<String, MigrationError> {
        scheme_from_settings_under_any_tracker(
            pattern,
            key,
            KeySource::LegacyDefaultProjectCode,
        )
        .map_err(|error| failure(&error))?
        .canonicalise(raw)
        .map_err(|error| failure(&error))
    }
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use migrate::ports::WorkItemIdentity as _;

    use super::WorkIdentity;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_bare_number_renders_under_the_key() -> Result<(), TestError> {
        assert_eq!(
            WorkIdentity.canonicalise(
                "7",
                "{project}-{number:04d}",
                Some("ACME")
            )?,
            "ACME-0007"
        );
        Ok(())
    }

    #[test]
    fn a_key_the_pattern_does_not_name_is_never_read() -> Result<(), TestError>
    {
        assert_eq!(
            WorkIdentity.canonicalise("7", "{number:04d}", Some("MY_PROJ"))?,
            "0007"
        );
        Ok(())
    }

    #[test]
    fn a_malformed_key_names_the_setting_it_came_from() {
        let refusal = WorkIdentity
            .canonicalise("7", "{project}-{number:04d}", Some("1X"))
            .err()
            .map(|error| error.to_string());

        assert!(
            refusal.as_deref().is_some_and(|text| text.starts_with(
                "E_WORK_KEY_INVALID: work.default_project_code '1X':"
            )),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_malformed_pattern_names_the_setting() {
        let refusal = WorkIdentity
            .canonicalise("7", "{project}-{number:4d}", Some("ACME"))
            .err()
            .map(|error| error.to_string());

        assert!(refusal.is_some_and(|text| text.starts_with(
            "E_WORK_ID_PATTERN_INVALID: work.id_pattern \
             '{project}-{number:4d}':"
        )));
    }
}
