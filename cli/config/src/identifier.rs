//! The shared lowercase-identifier validator guarding names that reach a path.

use crate::error::ConfigError;

/// Validates `name` as `^[a-z0-9][a-z0-9-]*$`, so a traversing or otherwise
/// unsafe value can never reach a filesystem path. `kind` names the identifier
/// in the error ("skill name", "template name").
///
/// # Errors
///
/// [`ConfigError::Invalid`] when `name` is not a valid identifier.
pub fn validate_identifier(kind: &str, name: &str) -> Result<(), ConfigError> {
    let mut chars = name.chars();
    let head_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let tail_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if head_ok && tail_ok {
        Ok(())
    } else {
        Err(ConfigError::Invalid {
            detail: format!("invalid {kind} '{name}'"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::validate_identifier;
    use crate::error::ConfigError;

    #[test]
    fn accepts_a_lowercase_identifier() {
        assert!(validate_identifier("skill name", "create-plan").is_ok());
        assert!(validate_identifier("skill name", "a1").is_ok());
    }

    #[test]
    fn rejects_leading_hyphen_uppercase_empty_and_embedded_space() {
        for bad in ["-x", "Ab", "", "a b"] {
            assert!(matches!(
                validate_identifier("template name", bad),
                Err(ConfigError::Invalid { .. })
            ));
        }
    }

    #[test]
    fn the_error_names_the_kind_and_value() {
        assert_eq!(
            validate_identifier("skill name", "Bad"),
            Err(ConfigError::Invalid {
                detail: "invalid skill name 'Bad'".to_owned()
            })
        );
        assert_eq!(
            validate_identifier("template name", "-x"),
            Err(ConfigError::Invalid {
                detail: "invalid template name '-x'".to_owned()
            })
        );
    }
}
