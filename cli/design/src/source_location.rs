//! What an inventory may be taken of: a URL, or a path in the repository.

use std::fmt;

use crate::host::Host;
use crate::host::HostError;

/// The scheme a URL location carries. Only the two the tool acts on are
/// modelled; every other scheme is a rejection at parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Https,
    Http,
}

/// A location the tool was asked to inspect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLocation {
    Url {
        scheme: Scheme,
        host: Host,
    },
    /// `about:blank` names no host and reaches no network. The shell
    /// classifies it as its own scheme and then accepts it by falling through
    /// every rejection, so it is modelled explicitly rather than left to
    /// emerge from the absence of a check.
    Blank,
    RepositoryPath(String),
}

/// Why a location string could not be interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationError {
    /// A scheme the tool does not act on.
    Scheme(String),
    /// A `../` component anywhere in a path location.
    PathEscape(String),
    /// The authority as written, and why it could not be canonicalised. The
    /// authority is carried so the message can name the host the shell named,
    /// which canonicalisation itself never produced for a rejected input.
    Host { authority: String, error: HostError },
}

impl std::error::Error for LocationError {}

impl fmt::Display for LocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scheme(scheme) => match scheme.as_str() {
                "file" => write!(
                    formatter,
                    "file:// URLs are not permitted as inventory locations."
                ),
                "javascript" => write!(
                    formatter,
                    "javascript: URLs are not permitted as inventory \
                     locations."
                ),
                "data" => write!(
                    formatter,
                    "data: URLs are not permitted as inventory locations."
                ),
                "chrome" => write!(
                    formatter,
                    "chrome:// URLs are not permitted as inventory locations."
                ),
                "about" => write!(
                    formatter,
                    "about: URLs (other than about:blank) are not permitted \
                     as inventory locations."
                ),
                other => write!(
                    formatter,
                    "scheme '{other}://' is not permitted. Only https:// and \
                     relative code-repo paths are accepted."
                ),
            },
            Self::PathEscape(location) => write!(
                formatter,
                "location '{location}' uses a ../ path escape, which is not \
                 permitted."
            ),
            Self::Host { authority, error } => {
                formatter.write_str(&error.message(authority))
            }
        }
    }
}

/// Interprets a raw location argument.
///
/// # Errors
///
/// A [`LocationError`] when the scheme is one the tool refuses, when a path
/// escapes upward, or when the authority cannot be canonicalised.
pub fn parse(location: &str) -> Result<SourceLocation, LocationError> {
    if location == "about:blank" {
        return Ok(SourceLocation::Blank);
    }
    for (prefix, scheme) in
        [("https://", Scheme::Https), ("http://", Scheme::Http)]
    {
        if let Some(rest) = location.strip_prefix(prefix) {
            let authority = authority_of(rest);
            let host = Host::canonicalise(authority).map_err(|error| {
                LocationError::Host {
                    authority: authority.to_owned(),
                    error,
                }
            })?;
            return Ok(SourceLocation::Url { scheme, host });
        }
    }
    for prefix in ["file://", "javascript:", "data:", "chrome://", "about:"] {
        if location.starts_with(prefix) {
            let name = prefix.trim_end_matches([':', '/']);
            return Err(LocationError::Scheme(name.to_owned()));
        }
    }
    if let Some((scheme, _)) = location.split_once("://") {
        return Err(LocationError::Scheme(scheme.to_owned()));
    }
    if location.contains("..") {
        return Err(LocationError::PathEscape(location.to_owned()));
    }
    Ok(SourceLocation::RepositoryPath(location.to_owned()))
}

/// The authority is everything before the first path, query or fragment
/// separator.
fn authority_of(rest: &str) -> &str {
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::LocationError;
    use super::Scheme;
    use super::SourceLocation;
    use crate::host::HostError;

    type TestError = Box<dyn std::error::Error>;

    fn scheme_of(location: &str) -> Result<Scheme, TestError> {
        match parse(location)? {
            SourceLocation::Url { scheme, .. } => Ok(scheme),
            other => Err(format!("expected a URL, got {other:?}").into()),
        }
    }

    fn host_of(location: &str) -> Result<String, TestError> {
        match parse(location)? {
            SourceLocation::Url { host, .. } => Ok(host.to_string()),
            other => Err(format!("expected a URL, got {other:?}").into()),
        }
    }

    #[test]
    fn each_url_scheme_the_tool_acts_on_parses() -> Result<(), TestError> {
        assert_eq!(scheme_of("https://example.com")?, Scheme::Https);
        assert_eq!(scheme_of("http://localhost:3000")?, Scheme::Http);
        Ok(())
    }

    #[test]
    fn the_authority_stops_at_the_path_query_or_fragment(
    ) -> Result<(), TestError> {
        for location in [
            "https://example.com/a/b",
            "https://example.com?q=1",
            "https://example.com#top",
        ] {
            assert_eq!(host_of(location)?, "example.com", "{location}");
        }
        Ok(())
    }

    /// The shell classifies `about:blank` as its own scheme and then accepts
    /// it by falling through every rejection. A reimplementation of the
    /// decision tree would plausibly turn that accept into a rejection.
    #[test]
    fn about_blank_is_accepted_while_every_other_about_url_is_not() {
        assert_eq!(parse("about:blank"), Ok(SourceLocation::Blank));
        assert_eq!(
            parse("about:config"),
            Err(LocationError::Scheme("about".to_owned()))
        );
    }

    #[test]
    fn every_refused_scheme_names_itself() {
        for (location, scheme) in [
            ("file:///etc/passwd", "file"),
            ("javascript:alert(1)", "javascript"),
            ("data:text/html,x", "data"),
            ("chrome://settings", "chrome"),
            ("ftp://example.com", "ftp"),
        ] {
            assert_eq!(
                parse(location),
                Err(LocationError::Scheme(scheme.to_owned())),
                "{location}"
            );
        }
    }

    #[test]
    fn a_path_escape_is_rejected_wherever_it_appears() {
        for location in ["../up", "./a/../b", "/abs/../b"] {
            assert_eq!(
                parse(location),
                Err(LocationError::PathEscape(location.to_owned())),
                "{location}"
            );
        }
    }

    #[test]
    fn a_plain_path_is_a_repository_path() {
        assert_eq!(
            parse("./src"),
            Ok(SourceLocation::RepositoryPath("./src".to_owned()))
        );
        assert_eq!(
            parse("src/pages"),
            Ok(SourceLocation::RepositoryPath("src/pages".to_owned()))
        );
    }

    #[test]
    fn a_host_rejection_travels_out_of_the_parse() -> Result<(), TestError> {
        assert_eq!(
            parse("https://user:pass@127.0.0.1@evil.com"),
            Err(LocationError::Host {
                authority: "user:pass@127.0.0.1@evil.com".to_owned(),
                error: HostError::Userinfo,
            })
        );
        assert_eq!(
            parse("https://0x7f000001"),
            Err(LocationError::Host {
                authority: "0x7f000001".to_owned(),
                error: HostError::NumericEncoding,
            })
        );
        let Err(numeric) = parse("https://0x7f000001") else {
            return Err("expected a numeric-encoding rejection".into());
        };
        assert!(numeric.to_string().contains("host '0x7f000001'"));
        Ok(())
    }
}
