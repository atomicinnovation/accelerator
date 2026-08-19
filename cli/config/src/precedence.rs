//! Environment-beats-configuration precedence for optional string settings.
//!
//! Pure over two `Option<&str>` values: the environment read stays in each
//! composition root (this crate holds no `std::env` read), and only the
//! decision — the first non-blank value wins, environment over configuration —
//! lives here, so it is tested without touching the process environment.

/// The effective value of an env-over-config optional setting.
///
/// The first non-blank value wins, environment over configuration; a blank
/// value (empty or whitespace-only) on either side is treated as absent, so a
/// blank environment override falls through to the configured value.
#[must_use]
pub fn env_beats_config(
    from_env: Option<&str>,
    from_config: Option<&str>,
) -> Option<String> {
    non_blank(from_env).or_else(|| non_blank(from_config))
}

fn non_blank(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::env_beats_config;

    #[test]
    fn the_environment_wins_when_both_are_set() {
        assert_eq!(
            env_beats_config(Some("/env"), Some("/config")),
            Some("/env".to_owned())
        );
    }

    #[test]
    fn the_config_value_is_used_when_the_environment_is_unset() {
        assert_eq!(
            env_beats_config(None, Some("/config")),
            Some("/config".to_owned())
        );
    }

    #[test]
    fn the_environment_value_is_used_when_the_config_is_unset() {
        assert_eq!(
            env_beats_config(Some("/env"), None),
            Some("/env".to_owned())
        );
    }

    #[test]
    fn neither_set_is_none() {
        assert_eq!(env_beats_config(None, None), None);
    }

    #[test]
    fn a_blank_environment_override_falls_through_to_the_config() {
        assert_eq!(
            env_beats_config(Some("   "), Some("/config")),
            Some("/config".to_owned())
        );
    }

    #[test]
    fn a_blank_value_on_both_sides_is_none() {
        assert_eq!(env_beats_config(Some(" "), Some("\t")), None);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(
            env_beats_config(Some("  /env  "), None),
            Some("/env".to_owned())
        );
    }

    #[test]
    fn a_blank_config_value_with_no_env_is_none() {
        assert_eq!(env_beats_config(None, Some("   ")), None);
    }
}
