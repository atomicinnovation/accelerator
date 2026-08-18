//! The second transport: a raw `PUT` of file bytes to a URL the **server**
//! nominated.
//!
//! It is the one request in this crate that talks to a host the client did not
//! choose, so its policy is deliberately different from the port transport's:
//! it sends no `Authorization`, refuses redirects, pins the scheme, bounds the
//! response, retries a bounded number of times, and confines the destination to
//! `*.linear.app` at a label boundary.
//!
//! The loopback exemption is a **constructor parameter**, never an environment
//! read: carried into a compiled binary as an env switch, the allowlist would
//! turn from a security control into an advisory one that any hostile repo hook
//! could disable.

use std::io::Read as _;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Url;

use crate::error::ClientError;
use crate::surface::SurfaceError;

/// The documented Linear uploads host, and the suffix every other permitted
/// host must carry at a label boundary.
const UPLOADS_HOST: &str = "uploads.linear.app";
const LINEAR_APP_SUFFIX: &str = ".linear.app";

/// The bounded PUT retry.
const MAX_ATTEMPTS: usize = 3;

/// The per-request timeout for the upload PUT. Deliberately longer than the
/// 30s port default: this is not a port operation, so the two do not
/// contradict.
pub const UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// The upload response bound. The PUT's response is small, but this is the one
/// request talking to a server-nominated host, so it is bounded like every
/// other.
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// A `key`/`value` header the `fileUpload` response echoed for the PUT to
/// carry.
#[derive(Debug, Clone)]
pub struct EchoedHeader {
    pub name: String,
    pub value: String,
}

pub struct UploadTransport {
    client: Client,
    allow_loopback: bool,
    retry_delay: Duration,
}

impl UploadTransport {
    /// Builds the upload transport.
    ///
    /// `allow_loopback` admits an `http://127.0.0.1|localhost` destination so
    /// tests can point the PUT at a cleartext mock; production passes `false`,
    /// and no environment variable can flip it. `retry_delay` is the wait
    /// between the bounded PUT attempts.
    ///
    /// # Errors
    ///
    /// [`ClientError::TlsUnavailable`] when the client cannot be built.
    pub fn new(
        allow_loopback: bool,
        retry_delay: Duration,
    ) -> Result<Self, ClientError> {
        install_crypto_provider();
        let client = Client::builder()
            .timeout(UPLOAD_TIMEOUT)
            // The PUT must never follow a redirect: an unfollowed 30x has to
            // surface rather than move file bytes to a host the server did not
            // originally nominate. With the scheme validated up front, refusing
            // redirects is the reqwest equivalent of curl's `--proto` pin.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClientError::TlsUnavailable {
                detail: error.to_string(),
            })?;
        Ok(Self {
            client,
            allow_loopback,
            retry_delay,
        })
    }

    /// The production transport: no loopback, a one-second wait between
    /// attempts.
    ///
    /// # Errors
    ///
    /// As [`UploadTransport::new`].
    pub fn production() -> Result<Self, ClientError> {
        Self::new(false, Duration::from_secs(1))
    }

    /// Whether the transport admits a loopback destination, so a caller can
    /// tell whether it is safe to point at a mock.
    #[must_use]
    pub const fn allows_loopback(&self) -> bool {
        self.allow_loopback
    }

    /// PUTs `bytes` to `url`, sending no `Authorization` and only the allowed
    /// echoed headers.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::BadUploadUrl`] if the destination is not allow-listed,
    /// [`SurfaceError::BadEchoedHeader`] if an allowed header carries a CR or
    /// LF, and [`SurfaceError::UploadFailed`] once the bounded retry is
    /// exhausted.
    pub fn put(
        &self,
        url: &str,
        content_type: &str,
        echoed: &[EchoedHeader],
        bytes: &[u8],
    ) -> Result<(), SurfaceError> {
        if !url_is_allowed(url, self.allow_loopback) {
            return Err(SurfaceError::BadUploadUrl {
                role: "uploadUrl",
                host: host_of(url),
            });
        }
        let headers = build_headers(content_type, echoed)?;

        let mut last = "no attempt made".to_owned();
        for attempt in 1..=MAX_ATTEMPTS {
            match self.attempt(url, &headers, bytes) {
                Ok(()) => return Ok(()),
                Err(detail) => {
                    last = detail;
                    if attempt < MAX_ATTEMPTS {
                        std::thread::sleep(self.retry_delay);
                    }
                }
            }
        }
        Err(SurfaceError::UploadFailed {
            url: redact(url),
            attempts: MAX_ATTEMPTS,
            detail: last,
        })
    }

    /// One PUT attempt: a 2xx is success, anything else is a retryable detail.
    fn attempt(
        &self,
        url: &str,
        headers: &[(String, String)],
        bytes: &[u8],
    ) -> Result<(), String> {
        let mut request = self.client.put(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .body(bytes.to_vec())
            .send()
            .map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        if let Some(length) = response.content_length() {
            if length > MAX_RESPONSE_BYTES as u64 {
                return Err(format!(
                    "the response declared {length} bytes, beyond the \
                     {MAX_RESPONSE_BYTES}-byte bound"
                ));
            }
        }
        drain(response);
        if (200..300).contains(&status) {
            return Ok(());
        }
        Err(format!("HTTP {status}"))
    }
}

/// Validates a server-supplied `uploadUrl` or `assetUrl` before any bytes move.
///
/// Accepts `https://uploads.linear.app` or any `*.linear.app` host matched at a
/// label boundary — a look-alike like `uploads.linear.app.evil.com` or
/// `evil-linear.app` is refused. An `http://127.0.0.1|localhost` destination is
/// admitted only when `allow_loopback` is set, which only tests do.
#[must_use]
pub(crate) fn url_is_allowed(url: &str, allow_loopback: bool) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    if allow_loopback && parsed.scheme() == "http" {
        if let Some(host) = parsed.host_str() {
            if host == "127.0.0.1" || host == "localhost" {
                return true;
            }
        }
    }
    if parsed.scheme() != "https" {
        return false;
    }
    // Userinfo would let `https://uploads.linear.app@evil.com` read as trusted
    // to a careless host split; the host below is `evil.com`, but refusing
    // userinfo outright removes the ambiguity.
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    host == UPLOADS_HOST || is_linear_app_subdomain(host)
}

/// A host is a `linear.app` subdomain only when it ends in `.linear.app` with a
/// non-empty label before it — the label boundary a bare `ends_with` misses.
fn is_linear_app_subdomain(host: &str) -> bool {
    host.strip_suffix(LINEAR_APP_SUFFIX)
        .is_some_and(|label| !label.is_empty())
}

/// The host of a URL for a diagnostic, or the raw string when it will not
/// parse.
fn host_of(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

/// Strips the signed query from a URL for a diagnostic: the query carries a
/// short-TTL bearer-grade capability that must not reach a log.
pub(crate) fn redact(url: &str) -> String {
    url.split_once('?')
        .map_or_else(|| url.to_owned(), |(base, _)| base.to_owned())
}

/// Content-Type and Cache-Control, plus the echoed headers filtered to the
/// `x-amz-*` allowlist. An allowed header carrying a CR or LF is a
/// header-injection vector and is refused rather than sent.
fn build_headers(
    content_type: &str,
    echoed: &[EchoedHeader],
) -> Result<Vec<(String, String)>, SurfaceError> {
    let mut headers = vec![
        ("Content-Type".to_owned(), content_type.to_owned()),
        (
            "Cache-Control".to_owned(),
            "public, max-age=31536000".to_owned(),
        ),
    ];
    for header in echoed {
        let lower = header.name.to_ascii_lowercase();
        if lower == "authorization" || lower == "host" {
            continue;
        }
        if !lower.starts_with("x-amz-") {
            continue;
        }
        if header.value.contains(['\r', '\n']) {
            return Err(SurfaceError::BadEchoedHeader {
                name: header.name.clone(),
            });
        }
        headers.push((header.name.clone(), header.value.clone()));
    }
    Ok(headers)
}

fn drain(response: reqwest::blocking::Response) {
    let mut sink = Vec::new();
    let mut bounded = response.take(MAX_RESPONSE_BYTES as u64);
    let _ = bounded.read_to_end(&mut sink);
}

fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const ALLOWLIST_FIXTURE: &str =
        include_str!("../tests/fixtures/upload-url-allowlist.txt");

    #[test]
    fn every_adversarial_url_resolves_as_the_fixture_expects() {
        let mut checked = 0;
        for line in ALLOWLIST_FIXTURE.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (verdict, url) =
                line.split_once(' ').expect("a verdict and a URL");
            let expected = match verdict {
                "accept" => true,
                "reject" => false,
                other => panic!("unknown verdict {other:?}"),
            };
            // The fixture is evaluated with loopback denied — production
            // semantics — so a look-alike cannot slip through a test escape.
            assert_eq!(
                url_is_allowed(url, false),
                expected,
                "case {url:?} expected {verdict}"
            );
            checked += 1;
        }
        assert!(
            checked >= 8,
            "the fixture must exercise every case, got {checked}"
        );
    }

    #[test]
    fn loopback_is_admitted_only_when_the_constructor_enables_it() {
        let loopback = "http://127.0.0.1:8080/upload";
        assert!(
            url_is_allowed(loopback, true),
            "a loopback destination is admitted for a test transport"
        );
        assert!(
            !url_is_allowed(loopback, false),
            "the production constructor refuses loopback regardless of \
             process environment"
        );
    }

    #[test]
    fn redaction_strips_the_signed_query() {
        assert_eq!(
            redact("https://uploads.linear.app/x?sig=secret&exp=1"),
            "https://uploads.linear.app/x"
        );
    }
}
