//! The `http(s)` URL of a page the web command may fetch, and the URL a
//! redirect from it leads to.

use std::fmt;

use network::Authority;
use network::AuthorityError;
use network::Host;
use network::HostError;

/// Which rule a URL broke. Each names its rule and never the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageUrlError {
    Scheme,
    Character,
    Host(HostError),
    Port,
}

impl std::error::Error for PageUrlError {}

impl fmt::Display for PageUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Scheme => "only http and https are fetched",
            Self::Character => {
                "the URL contains whitespace, a control character or a \
                 backslash"
            }
            Self::Host(HostError::Userinfo) => {
                "the URL carries user information"
            }
            Self::Host(HostError::Empty) => "the URL has no host",
            Self::Host(HostError::ControlCharacter) => {
                "the host contains a control character"
            }
            Self::Host(HostError::NumericEncoding) => {
                "the host is a numerically encoded address"
            }
            Self::Port => "the port is not a number from 1 to 65535",
        })
    }
}

impl From<AuthorityError> for PageUrlError {
    fn from(error: AuthorityError) -> Self {
        match error {
            AuthorityError::Host(error) => Self::Host(error),
            AuthorityError::Port => Self::Port,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageScheme {
    Http,
    Https,
}

impl PageScheme {
    const fn parse(raw: &str) -> Option<Self> {
        if raw.eq_ignore_ascii_case("http") {
            Some(Self::Http)
        } else if raw.eq_ignore_ascii_case("https") {
            Some(Self::Https)
        } else {
            None
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    const fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

/// A page URL with its host canonicalised and its fragment dropped.
///
/// A Unicode host is kept as written, lowercased: its ASCII form is the
/// adapters' to compute. WHATWG parsers read `\` as `/`, so a backslash
/// anywhere is refused rather than left for a parser to read a different
/// host from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUrl {
    scheme: PageScheme,
    host: Host,
    port: u16,
    path_and_query: String,
}

impl PageUrl {
    /// # Errors
    ///
    /// The [`PageUrlError`] naming the first rule the URL broke.
    pub fn parse(raw: &str) -> Result<Self, PageUrlError> {
        let (scheme, rest) = raw
            .split_once(':')
            .and_then(|(scheme, rest)| Some((PageScheme::parse(scheme)?, rest)))
            .ok_or(PageUrlError::Scheme)?;
        if raw
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
        {
            return Err(PageUrlError::Character);
        }
        let hierarchy = rest
            .strip_prefix("//")
            .ok_or(PageUrlError::Host(HostError::Empty))?;
        let unfragmented = hierarchy.split('#').next().unwrap_or_default();
        let authority_end =
            unfragmented.find(['/', '?']).unwrap_or(unfragmented.len());
        let (authority, path_and_query) = unfragmented.split_at(authority_end);
        let authority = Authority::parse(authority)?;
        Ok(Self {
            scheme,
            port: authority.port().unwrap_or_else(|| scheme.default_port()),
            host: authority.host().clone(),
            path_and_query: if path_and_query.starts_with('/') {
                path_and_query.to_owned()
            } else {
                format!("/{path_and_query}")
            },
        })
    }

    /// The URL a redirect's `location` leads to, resolved against this one
    /// as RFC 3986 §5.2 resolves a reference, then held to every rule
    /// [`Self::parse`] holds a URL to.
    ///
    /// # Errors
    ///
    /// The [`PageUrlError`] naming the first rule the resolved URL broke.
    pub fn follow(&self, location: &str) -> Result<Self, PageUrlError> {
        let reference = location.split('#').next().unwrap_or_default();
        if has_scheme(reference) {
            return Self::parse(reference);
        }
        if reference.starts_with("//") {
            return Self::parse(&format!("{}:{reference}", self.scheme.name()));
        }
        let (path, _) = split_query(&self.path_and_query);
        let target = if reference.is_empty() {
            self.path_and_query.clone()
        } else if reference.starts_with('?') {
            format!("{path}{reference}")
        } else if reference.starts_with('/') {
            without_dot_segments(reference)
        } else {
            let directory = &path[..=path.rfind('/').unwrap_or_default()];
            without_dot_segments(&format!("{directory}{reference}"))
        };
        Self::parse(&format!("{}{target}", self.origin()))
    }

    pub const fn host(&self) -> &Host {
        &self.host
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    fn origin(&self) -> String {
        let host =
            if self.host.address().is_some_and(|address| address.is_ipv6()) {
                format!("[{}]", self.host)
            } else {
                self.host.to_string()
            };
        let port = if self.port == self.scheme.default_port() {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        format!("{}://{host}{port}", self.scheme.name())
    }
}

impl fmt::Display for PageUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.origin(), self.path_and_query)
    }
}

fn has_scheme(reference: &str) -> bool {
    reference.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')
            })
    })
}

fn split_query(path_and_query: &str) -> (&str, &str) {
    path_and_query
        .find('?')
        .map_or((path_and_query, ""), |at| path_and_query.split_at(at))
}

/// RFC 3986 §5.2.4 over an absolute path, carrying any query through
/// untouched.
fn without_dot_segments(path_and_query: &str) -> String {
    let (path, query) = split_query(path_and_query);
    let segments = path
        .strip_prefix('/')
        .unwrap_or(path)
        .split('/')
        .collect::<Vec<_>>();
    let last = segments.len() - 1;
    let mut kept = Vec::with_capacity(segments.len());
    for (index, segment) in segments.into_iter().enumerate() {
        match segment {
            "." | ".." => {
                if segment == ".." {
                    kept.pop();
                }
                if index == last {
                    kept.push("");
                }
            }
            other => kept.push(other),
        }
    }
    format!("/{}{query}", kept.join("/"))
}
