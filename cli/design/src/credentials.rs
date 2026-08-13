//! Which authentication mode the configured environment selects.
//!
//! The `header` mode is inert downstream: the daemon imports its handler and
//! never calls it, and the origin allowlist that handler needs is set nowhere.
//! The mode still resolves, and the command's help text warns callers, because
//! it remains a documented capability.

use std::fmt;

/// The environment values the modes are selected from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credentials {
    pub auth_header: Option<String>,
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

/// The variables a partial form-login configuration is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialConfiguration {
    pub missing: Vec<&'static str>,
}

impl std::error::Error for PartialConfiguration {}

impl fmt::Display for PartialConfiguration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "partial form-login configuration — missing: {}. Set all three of \
             ACCELERATOR_BROWSER_USERNAME, ACCELERATOR_BROWSER_PASSWORD, and \
             ACCELERATOR_BROWSER_LOGIN_URL together, or use \
             ACCELERATOR_BROWSER_AUTH_HEADER instead.",
            self.missing.join(", ")
        )
    }
}

const USERNAME: &str = "ACCELERATOR_BROWSER_USERNAME";
const PASSWORD: &str = "ACCELERATOR_BROWSER_PASSWORD";
const LOGIN_URL: &str = "ACCELERATOR_BROWSER_LOGIN_URL";

/// Resolves the mode `credentials` selects.
///
/// # Errors
///
/// A [`PartialConfiguration`] naming the missing variables when some but not
/// all of the form-login trio are set and no header is configured.
pub fn resolve(
    credentials: &Credentials,
) -> Result<Resolution, PartialConfiguration> {
    let form = [
        (USERNAME, &credentials.username),
        (PASSWORD, &credentials.password),
        (LOGIN_URL, &credentials.login_url),
    ];

    if credentials.auth_header.is_some() {
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
        _ => Err(PartialConfiguration { missing }),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve;
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
    fn a_header_wins_over_a_complete_form_configuration() -> Result<(), String>
    {
        let credentials = Credentials {
            auth_header: Some(set("Authorization: Bearer abc123")),
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
    fn a_header_alone_warns_about_nothing() -> Result<(), String> {
        let credentials = Credentials {
            auth_header: Some(set("Authorization: Bearer abc123")),
            ..Credentials::default()
        };
        let resolution =
            resolve(&credentials).map_err(|error| error.to_string())?;
        assert_eq!(resolution.mode, AuthMode::Header);
        assert_eq!(resolution.warning, None);
        Ok(())
    }

    #[test]
    fn a_partial_form_configuration_names_every_missing_variable(
    ) -> Result<(), String> {
        let credentials = Credentials {
            username: Some(set("alice")),
            ..Credentials::default()
        };
        let Err(error) = resolve(&credentials) else {
            return Err("expected a refusal".to_owned());
        };
        assert_eq!(
            error.missing,
            vec![
                "ACCELERATOR_BROWSER_PASSWORD",
                "ACCELERATOR_BROWSER_LOGIN_URL"
            ]
        );
        assert!(error
            .to_string()
            .contains("partial form-login configuration"));
        Ok(())
    }

    #[test]
    fn each_mode_prints_the_word_the_skill_reads() {
        assert_eq!(AuthMode::Header.to_string(), "header");
        assert_eq!(AuthMode::Form.to_string(), "form");
        assert_eq!(AuthMode::None.to_string(), "none");
    }
}
