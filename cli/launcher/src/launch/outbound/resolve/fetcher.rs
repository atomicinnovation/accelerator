//! Blocking reqwest fetch with timeouts, retry, and a redirect host-allowlist
//! matched at a dotted-label boundary (so `evil-githubusercontent.com` and
//! `githubusercontent.com.attacker.net` are refused).

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::redirect::{Attempt, Policy};

const MAX_ATTEMPTS: u32 = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
// Whole-request deadline per attempt: blocking reqwest has no idle/read-stall
// timeout, so this is the only bound on a slow-but-progressing transfer. Sized
// for a multi-MB release binary over a slow link.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_REDIRECTS: usize = 10;
const CDN_HOST_SUFFIX: &str = ".githubusercontent.com";
const RELEASE_ORIGIN_HOST: &str = "github.com";

/// Why a fetch did not yield bytes. A definitive 404 is not retried; a transport
/// error or exhausted 5xx retries is `Unreachable`.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchError {
    NotFound,
    Unreachable(String),
}

/// Whether a redirect target host is permitted, matched at a dotted-label
/// boundary.
#[must_use]
pub fn is_allowed_redirect_host(host: &str) -> bool {
    host == RELEASE_ORIGIN_HOST || host.ends_with(CDN_HOST_SUFFIX)
}

/// Whether a URL uses the required `https` scheme.
#[must_use]
pub fn is_https(url: &str) -> bool {
    url.starts_with("https://")
}

fn redirect_policy() -> Policy {
    Policy::custom(|attempt: Attempt| {
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        match attempt.url().host_str() {
            Some(host) if is_allowed_redirect_host(host) => attempt.follow(),
            _ => attempt.stop(),
        }
    })
}

/// A configured blocking HTTP client for asset/manifest fetches.
pub struct Fetcher {
    client: Client,
    max_attempts: u32,
    backoff: Duration,
    require_https: bool,
}

impl Fetcher {
    /// Build the production fetcher (https pinned).
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn new() -> Result<Self, String> {
        Self::build(Duration::from_millis(250), true)
    }

    /// Build a test fetcher with a caller-chosen backoff, permitting `http` for
    /// a local mock server.
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn with_backoff(backoff: Duration) -> Result<Self, String> {
        Self::build(backoff, false)
    }

    fn build(backoff: Duration, require_https: bool) -> Result<Self, String> {
        // The ring provider must be installed before building a TLS client
        // (idempotent), keeping the Fetcher self-sufficient in tests.
        let _ = rustls::crypto::ring::default_provider().install_default();
        // `https_only` re-enforces the scheme on the post-redirect URL too, not
        // just the initial request the `get()` guard checks. Off for the test
        // fetcher so it can reach a local `http` mock.
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .redirect(redirect_policy())
            .https_only(require_https)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            max_attempts: MAX_ATTEMPTS,
            backoff,
            require_https,
        })
    }

    /// GET `url`, retrying transient/5xx failures up to the attempt cap; a 404
    /// returns immediately and a production fetcher refuses non-https up front.
    ///
    /// # Errors
    ///
    /// [`FetchError`] describing the terminal failure.
    pub fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        if self.require_https && !is_https(url) {
            return Err(FetchError::Unreachable(format!(
                "refusing non-https URL (scheme not permitted): {url}"
            )));
        }
        let mut last = String::new();
        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                std::thread::sleep(self.backoff * attempt);
            }
            match self.try_get(url) {
                Ok(bytes) => return Ok(bytes),
                Err(Terminal::NotFound) => return Err(FetchError::NotFound),
                Err(Terminal::Retryable(detail)) => last = detail,
            }
        }
        Err(FetchError::Unreachable(last))
    }

    fn try_get(&self, url: &str) -> Result<Vec<u8>, Terminal> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| Terminal::Retryable(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(Terminal::NotFound);
        }
        if status.is_server_error() {
            return Err(Terminal::Retryable(format!("server error {status}")));
        }
        if !status.is_success() {
            return Err(Terminal::Retryable(format!(
                "unexpected status {status}"
            )));
        }
        response
            .bytes()
            .map(|body| body.to_vec())
            .map_err(|error| Terminal::Retryable(error.to_string()))
    }
}

enum Terminal {
    NotFound,
    Retryable(String),
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_redirect_host, is_https, FetchError, Fetcher};

    #[test]
    fn production_fetcher_refuses_non_https_urls() {
        let Ok(fetcher) = Fetcher::new() else {
            return;
        };
        let result = fetcher.get("http://127.0.0.1:1/asset");
        assert!(
            matches!(
                &result,
                Err(FetchError::Unreachable(detail))
                    if detail.contains("https")
            ),
            "expected an https scheme refusal, got {result:?}"
        );
    }

    #[test]
    fn cdn_suffix_and_origin_are_allowed_redirect_hosts() {
        assert!(is_allowed_redirect_host("github.com"));
        assert!(is_allowed_redirect_host(
            "objects.release.githubusercontent.com"
        ));
    }

    #[test]
    fn lookalike_hosts_are_refused_redirect_targets() {
        assert!(!is_allowed_redirect_host("evil.example.com"));
        assert!(!is_allowed_redirect_host("evil-githubusercontent.com"));
        assert!(!is_allowed_redirect_host(
            "githubusercontent.com.attacker.net"
        ));
    }

    #[test]
    fn https_is_required_by_the_production_pin() {
        assert!(is_https("https://github.com/x"));
        assert!(!is_https("http://github.com/x"));
    }
}
