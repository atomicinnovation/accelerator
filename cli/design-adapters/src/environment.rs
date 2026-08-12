//! The `ACCELERATOR_BROWSER_*` reads.
//!
//! An unset variable and one set to the empty string are the same thing here,
//! matching the shell's `${VAR:-}` idiom: an exported-but-blank credential
//! configures nothing.

use design::leaked_credentials::NamedSecret;
use design::Credentials;

const AUTH_HEADER: &str = "ACCELERATOR_BROWSER_AUTH_HEADER";
const USERNAME: &str = "ACCELERATOR_BROWSER_USERNAME";
const PASSWORD: &str = "ACCELERATOR_BROWSER_PASSWORD";
const LOGIN_URL: &str = "ACCELERATOR_BROWSER_LOGIN_URL";

/// Every variable the credential vocabulary covers, in the order the shell
/// scanned them.
pub const CREDENTIAL_VARIABLES: [&str; 4] =
    [AUTH_HEADER, USERNAME, PASSWORD, LOGIN_URL];

fn non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The configured credentials, as the auth-mode resolution reads them.
#[must_use]
pub fn credentials_from_env() -> Credentials {
    Credentials {
        auth_header: non_empty(AUTH_HEADER),
        username: non_empty(USERNAME),
        password: non_empty(PASSWORD),
        login_url: non_empty(LOGIN_URL),
    }
}

/// The configured credentials, as the leak scan reads them.
#[must_use]
pub fn named_secrets_from_env() -> Vec<NamedSecret> {
    CREDENTIAL_VARIABLES
        .iter()
        .filter_map(|name| {
            non_empty(name).map(|value| NamedSecret {
                name: (*name).to_owned(),
                value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::CREDENTIAL_VARIABLES;

    #[test]
    fn the_variable_set_matches_the_scrubber_s_own_vocabulary() {
        assert_eq!(
            CREDENTIAL_VARIABLES,
            [
                "ACCELERATOR_BROWSER_AUTH_HEADER",
                "ACCELERATOR_BROWSER_USERNAME",
                "ACCELERATOR_BROWSER_PASSWORD",
                "ACCELERATOR_BROWSER_LOGIN_URL",
            ]
        );
    }
}
