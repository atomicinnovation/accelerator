//! Which authentication mode the configured environment selects.
//!
//! `header` mode injects `ACCELERATOR_BROWSER_AUTH_HEADER` on requests whose
//! origin matches the crawl's declared `ACCELERATOR_BROWSER_LOCATION` origin and
//! strips it on every cross-origin request; the daemon enforces the strip in
//! code. The two variables are a pair: the header keys to the location origin,
//! so a header set without a location is refused loudly rather than proceeding
//! into a stripped, unauthenticated crawl.

use std::fmt;

/// The environment values the modes are selected from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credentials {
    pub auth_header: Option<String>,
    pub location: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub login_url: Option<String>,
}

/// The mode a crawl authenticates with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Header,
    Form,
    None,
}

impl fmt::Display for AuthMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Header => "header",
            Self::Form => "form",
            Self::None => "none",
        })
    }
}

/// A resolved mode, plus whatever the caller should be told about the
/// configuration that selected it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub mode: AuthMode,
    /// Set when the header mode wins while form-login variables are also
    /// configured, naming the ones being ignored.
    pub warning: Option<String>,
}

/// Why the configured environment names no actionable auth mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthConfigurationError {
    /// Some but not all of the form-login trio are set, and no header is
    /// configured.
    PartialForm { missing: Vec<&'static str> },
    /// A header is set without the location that keys it.
    HeaderWithoutLocation,
}

impl std::error::Error for AuthConfigurationError {}

impl fmt::Display for AuthConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PartialForm { missing } => write!(
                formatter,
                "partial form-login configuration — missing: {}. Set all three \
                 of ACCELERATOR_BROWSER_USERNAME, ACCELERATOR_BROWSER_PASSWORD, \
                 and ACCELERATOR_BROWSER_LOGIN_URL together, or use \
                 ACCELERATOR_BROWSER_AUTH_HEADER with ACCELERATOR_BROWSER_LOCATION \
                 instead.",
                missing.join(", ")
            ),
            Self::HeaderWithoutLocation => write!(
                formatter,
                "ACCELERATOR_BROWSER_AUTH_HEADER is set without \
                 ACCELERATOR_BROWSER_LOCATION. Header mode keys the auth header \
                 to the crawl's location origin, so both must be set together; \
                 set ACCELERATOR_BROWSER_LOCATION to the crawl's [location] URL."
            ),
        }
    }
}

const USERNAME: &str = "ACCELERATOR_BROWSER_USERNAME";
const PASSWORD: &str = "ACCELERATOR_BROWSER_PASSWORD";
const LOGIN_URL: &str = "ACCELERATOR_BROWSER_LOGIN_URL";

/// Resolves the mode `credentials` selects.
///
/// # Errors
///
/// [`AuthConfigurationError::HeaderWithoutLocation`] when a header is set
/// without the location that keys it, or
/// [`AuthConfigurationError::PartialForm`] naming the missing variables when
/// some but not all of the form-login trio are set and no header is configured.
pub fn resolve(
    credentials: &Credentials,
) -> Result<Resolution, AuthConfigurationError> {
    let form = [
        (USERNAME, &credentials.username),
        (PASSWORD, &credentials.password),
        (LOGIN_URL, &credentials.login_url),
    ];

    if credentials.auth_header.is_some() {
        if credentials.location.is_none() {
            return Err(AuthConfigurationError::HeaderWithoutLocation);
        }
        let ignored: Vec<&str> = form
            .iter()
            .filter(|(_, value)| value.is_some())
            .map(|(name, _)| *name)
            .collect();
        let warning = (!ignored.is_empty()).then(|| {
            format!(
                "ACCELERATOR_BROWSER_AUTH_HEADER is set; form-login vars \
                 ignored: {}",
                ignored.join(", ")
            )
        });
        return Ok(Resolution {
            mode: AuthMode::Header,
            warning,
        });
    }

    let missing: Vec<&'static str> = form
        .iter()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();

    match missing.len() {
        0 => Ok(Resolution {
            mode: AuthMode::Form,
            warning: None,
        }),
        3 => Ok(Resolution {
            mode: AuthMode::None,
            warning: None,
        }),
        _ => Err(AuthConfigurationError::PartialForm { missing }),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use super::AuthConfigurationError;
    use super::AuthMode;
    use super::Credentials;

    fn set(value: &str) -> String {
        value.to_owned()
    }

    #[test]
    fn nothing_configured_resolves_to_none() -> Result<(), String> {
        let resolution = resolve(&Credentials::default())
            .map_err(|error| error.to_string())?;
        assert_eq!(resolution.mode, AuthMode::None);
        assert_eq!(resolution.warning, None);
        Ok(())
    }

    #[test]
    fn all_three_form_variables_resolve_to_form() -> Result<(), String> {
        let credentials = Credentials {
            username: Some(set("alice")),
            password: Some(set("hunter2")),
            login_url: Some(set("https://example.com/login")),
            ..Credentials::default()
        };
        let resolution =
            resolve(&credentials).map_err(|error| error.to_string())?;
        assert_eq!(resolution.mode, AuthMode::Form);
        Ok(())
    }

    #[test]
    fn a_header_with_a_location_wins_over_a_complete_form_configuration(
    ) -> Result<(), String> {
        let credentials = Credentials {
            auth_header: Some(set("Authorization: Bearer abc123")),
            location: Some(set("https://example.com")),
            username: Some(set("alice")),
            password: Some(set("hunter2")),
            login_url: Some(set("https://example.com/login")),
        };
        let resolution =
            resolve(&credentials).map_err(|error| error.to_string())?;
        assert_eq!(resolution.mode, AuthMode::Header);
        let warning = resolution.warning.ok_or("expected a warning")?;
        assert!(warning.contains("ACCELERATOR_BROWSER_USERNAME"));
        assert!(warning.contains("ACCELERATOR_BROWSER_PASSWORD"));
        assert!(warning.contains("ACCELERATOR_BROWSER_LOGIN_URL"));
        Ok(())
    }

    #[test]
    fn a_header_with_a_location_and_nothing_else_warns_about_nothing(
    ) -> Result<(), String> {
        let credentials = Credentials {
            auth_header: Some(set("Authorization: Bearer abc123")),
            location: Some(set("https://example.com")),
            ..Credentials::default()
        };
        let resolution =
            resolve(&credentials).map_err(|error| error.to_string())?;
        assert_eq!(resolution.mode, AuthMode::Header);
        assert_eq!(resolution.warning, None);
        Ok(())
    }

    #[test]
    fn a_header_without_a_location_is_refused_loudly() -> Result<(), String> {
        let credentials = Credentials {
            auth_header: Some(set("Authorization: Bearer abc123")),
            ..Credentials::default()
        };
        let Err(error) = resolve(&credentials) else {
            return Err("expected a refusal".to_owned());
        };
        assert_eq!(error, AuthConfigurationError::HeaderWithoutLocation);
        assert!(error.to_string().contains("ACCELERATOR_BROWSER_LOCATION"));
        Ok(())
    }

    #[test]
    fn a_partial_form_configuration_names_every_missing_variable(
    ) -> Result<(), String> {
        let credentials = Credentials {
            username: Some(set("alice")),
            ..Credentials::default()
        };
        let Err(AuthConfigurationError::PartialForm { missing }) =
            resolve(&credentials)
        else {
            return Err("expected a partial-form refusal".to_owned());
        };
        assert_eq!(
            missing,
            vec![
                "ACCELERATOR_BROWSER_PASSWORD",
                "ACCELERATOR_BROWSER_LOGIN_URL"
            ]
        );
        Ok(())
    }

    #[test]
    fn each_mode_prints_the_word_the_skill_reads() {
        assert_eq!(AuthMode::Header.to_string(), "header");
        assert_eq!(AuthMode::Form.to_string(), "form");
        assert_eq!(AuthMode::None.to_string(), "none");
    }
}
