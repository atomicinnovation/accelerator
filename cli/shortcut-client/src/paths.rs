//! Every v4 path the client requests, in one place: v4 is a beta, and a
//! breaking change to its paths should touch one module.
//!
//! Every value from outside the client — the workspace slug, an id — is added
//! as a path segment, never spliced into a string, so a value holding `/`, `?`
//! or `#` cannot point an authenticated request at another endpoint.

use reqwest::Url;

use crate::shortcut_key::Collection;
use crate::shortcut_key::ShortcutKey;

pub const PRODUCTION_BASE: &str = "https://api.app.shortcut.com";
const PRODUCTION_HOST: &str = "api.app.shortcut.com";
const API_PREFIX: [&str; 2] = ["api", "v4"];

/// Whether `url` may serve as the API base.
///
/// The production host is admitted over https only, at its default port, with
/// no userinfo and nothing after the origin. A loopback base is admitted only
/// when `allow_loopback`, which a composition root takes from its
/// `test-loopback` feature.
#[must_use]
pub fn url_is_allowed(url: &str, allow_loopback: bool) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let origin_only = parsed.username().is_empty()
        && parsed.password().is_none()
        && matches!(parsed.path(), "" | "/")
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    if !origin_only {
        return false;
    }
    if allow_loopback
        && parsed.scheme() == "http"
        && matches!(parsed.host_str(), Some("127.0.0.1" | "localhost"))
    {
        return true;
    }
    parsed.scheme() == "https"
        && parsed.host_str() == Some(PRODUCTION_HOST)
        && parsed.port().is_none()
}

/// A workspace's URL slug, as whoami reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSlug(String);

/// A slug outside `^[a-z0-9][a-z0-9-]*$`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafeSlug(pub String);

impl std::fmt::Display for UnsafeSlug {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:?} is not a workspace slug — expected lowercase letters, \
             digits and hyphens",
            self.0
        )
    }
}

impl std::error::Error for UnsafeSlug {}

impl WorkspaceSlug {
    /// # Errors
    ///
    /// [`UnsafeSlug`] for a slug outside `^[a-z0-9][a-z0-9-]*$`.
    pub fn parse(text: &str) -> Result<Self, UnsafeSlug> {
        let allowed = |byte: u8| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        };
        let valid = text.bytes().next().is_some_and(|first| first != b'-')
            && text.bytes().all(allowed);
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(UnsafeSlug(text.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The v4 paths under one workspace.
#[derive(Debug, Clone)]
pub struct Paths {
    base: Url,
    slug: WorkspaceSlug,
}

impl Paths {
    #[must_use]
    pub const fn new(base: Url, slug: WorkspaceSlug) -> Self {
        Self { base, slug }
    }

    #[must_use]
    pub const fn base(&self) -> &Url {
        &self.base
    }

    /// The one path that names no workspace: it is how the client learns the
    /// slug.
    #[must_use]
    pub fn whoami(base: &Url) -> Url {
        under(base, &["whoami"])
    }

    #[must_use]
    pub fn entity(&self, key: ShortcutKey) -> Url {
        self.workspace(&[
            key.collection().path_segment(),
            &key.number().to_string(),
        ])
    }

    #[must_use]
    pub fn comments(&self, key: ShortcutKey) -> Url {
        self.workspace(&[
            key.collection().path_segment(),
            &key.number().to_string(),
            "comments",
        ])
    }

    #[must_use]
    pub fn collection(&self, collection: Collection) -> Url {
        self.workspace(&[collection.path_segment()])
    }

    #[must_use]
    pub fn search_stories(&self) -> Url {
        self.workspace(&["stories", "search"])
    }

    #[must_use]
    pub fn teams(&self) -> Url {
        self.workspace(&["teams"])
    }

    #[must_use]
    pub fn team(&self, uuid: &str) -> Url {
        self.workspace(&["teams", uuid])
    }

    #[must_use]
    pub fn workflows(&self) -> Url {
        self.workspace(&["workflows"])
    }

    #[must_use]
    pub fn members(&self) -> Url {
        self.workspace(&["members"])
    }

    /// Whether a `next_page_url` Shortcut returned may be followed with the
    /// bearer token: same scheme, host and port as the base, and a path under
    /// `/api/v4/`.
    #[must_use]
    pub fn admits_next_page(&self, next: &str) -> bool {
        let Ok(parsed) = Url::parse(next) else {
            return false;
        };
        let same_origin = parsed.scheme() == self.base.scheme()
            && parsed.host_str() == self.base.host_str()
            && parsed.port_or_known_default()
                == self.base.port_or_known_default()
            && parsed.username().is_empty()
            && parsed.password().is_none();
        let mut segments = parsed.path_segments().into_iter().flatten();
        let under_api = API_PREFIX
            .iter()
            .all(|expected| segments.next() == Some(*expected));
        same_origin && under_api
    }

    fn workspace(&self, rest: &[&str]) -> Url {
        let mut segments = vec![self.slug.as_str()];
        segments.extend_from_slice(rest);
        under(&self.base, &segments)
    }
}

fn under(base: &Url, rest: &[&str]) -> Url {
    let mut url = base.clone();
    url.set_query(None);
    url.set_fragment(None);
    if let Ok(mut path) = url.path_segments_mut() {
        path.clear().extend(API_PREFIX).extend(rest);
    }
    url
}
