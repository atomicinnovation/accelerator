//! The dotted configuration key and its only constructor.

use std::fmt::{self, Display, Formatter};

use crate::error::ConfigError;

/// A dot-separated path into the configuration tree, held as its segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key(Vec<String>);

impl Key {
    /// Splits a raw dotted string into segments, rejecting any degenerate form
    /// — an empty key, a leading or trailing dot, or consecutive dots.
    ///
    /// # Errors
    ///
    /// [`ConfigError::InvalidKey`] when the raw key contains an empty segment.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let segments: Vec<String> = raw.split('.').map(str::to_owned).collect();
        if segments.iter().any(String::is_empty) {
            return Err(ConfigError::InvalidKey {
                key: raw.to_owned(),
            });
        }
        Ok(Self(segments))
    }

    pub(crate) fn segments(&self) -> &[String] {
        &self.0
    }
}

impl Display for Key {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.join("."))
    }
}

#[cfg(test)]
mod tests {
    use super::Key;
    use crate::error::ConfigError;

    #[test]
    fn parses_a_single_segment_key() -> Result<(), ConfigError> {
        let key = Key::parse("example")?;
        assert_eq!(key.segments(), ["example"]);
        Ok(())
    }

    #[test]
    fn parses_a_two_segment_key() -> Result<(), ConfigError> {
        let key = Key::parse("core.example")?;
        assert_eq!(key.segments(), ["core", "example"]);
        Ok(())
    }

    #[test]
    fn parses_a_deep_key() -> Result<(), ConfigError> {
        let key = Key::parse("a.b.c.d")?;
        assert_eq!(key.segments(), ["a", "b", "c", "d"]);
        Ok(())
    }

    #[test]
    fn rejects_the_empty_key() {
        assert_eq!(
            Key::parse(""),
            Err(ConfigError::InvalidKey { key: String::new() })
        );
    }

    #[test]
    fn rejects_a_leading_dot() {
        assert_eq!(
            Key::parse(".example"),
            Err(ConfigError::InvalidKey {
                key: ".example".to_owned()
            })
        );
    }

    #[test]
    fn rejects_a_trailing_dot() {
        assert_eq!(
            Key::parse("core."),
            Err(ConfigError::InvalidKey {
                key: "core.".to_owned()
            })
        );
    }

    #[test]
    fn rejects_consecutive_dots() {
        assert_eq!(
            Key::parse("core..example"),
            Err(ConfigError::InvalidKey {
                key: "core..example".to_owned()
            })
        );
    }

    #[test]
    fn displays_as_the_dotted_form() -> Result<(), ConfigError> {
        assert_eq!(Key::parse("a.b.c")?.to_string(), "a.b.c");
        Ok(())
    }
}
