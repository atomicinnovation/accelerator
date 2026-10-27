//! Fetching one public page for its declared metadata.
//!
//! Every hop's host is resolved and admitted before the request, the request
//! goes only to the addresses admitted, and redirects are followed within a
//! fixed count and the call's deadline.

use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;
use std::time::Instant;

use network::host_reach::classify_address;
use network::Host;
use network::HostReach;

use crate::sources::fetch::Clock;
use crate::sources::metadata::date::PartialDate;
use crate::sources::metadata::SourceMetadata;
use crate::sources::schedule::Deadline;
use crate::sources::web::declarations::PageDecoder;
use crate::sources::web::extract::declared_metadata;
use crate::sources::web::page_url::PageUrl;

pub const PAGE_ACCEPT: &str = "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8";
/// The declarations sit in the head, so a longer page is truncated rather
/// than refused.
pub const MAX_PAGE_BODY: usize = 2 * 1024 * 1024;
pub const LOOKUP_CAP: Duration = Duration::from_secs(5);
pub const MAX_REDIRECTS: usize = 5;

const HTML_MEDIA_TYPES: [&str; 2] = ["text/html", "application/xhtml+xml"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveFailure {
    Unresolved,
    /// The resolver's own guards refused to look the name up.
    Refused,
}

pub trait HostResolver {
    /// Every address `host` resolves to within `within`, each carrying
    /// `port`.
    ///
    /// # Errors
    ///
    /// [`ResolveFailure::Unresolved`] when the name could not be resolved in
    /// time.
    fn resolve(
        &self,
        host: &Host,
        port: u16,
        within: Duration,
    ) -> Result<Vec<SocketAddr>, ResolveFailure>;
}

/// One `GET`, which must connect only to `addresses`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest<'a> {
    pub url: &'a PageUrl,
    pub addresses: &'a [SocketAddr],
    pub accept: &'static str,
    pub max_body: usize,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageResponse {
    Received {
        status: u16,
        location: Option<String>,
        content_type: Option<String>,
        body: Vec<u8>,
    },
    /// The transport's own guards refused to send the request.
    Refused,
    /// No client could be built, so nothing was sent.
    Unavailable,
    ConnectionFailed,
    TimedOut,
}

pub trait PageTransport {
    fn get(&self, request: &PageRequest<'_>) -> PageResponse;
}

pub struct WebPorts<'a> {
    pub resolver: &'a dyn HostResolver,
    pub transport: &'a dyn PageTransport,
    pub decoder: &'a dyn PageDecoder,
    pub clock: &'a dyn Clock,
}

/// A host reach that is never admitted: every reach but
/// [`HostReach::Public`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectedReach {
    Loopback,
    Private,
    LinkLocal,
    Reserved,
    Unspecified,
}

impl RejectedReach {
    pub const fn of(reach: HostReach) -> Option<Self> {
        match reach {
            HostReach::Loopback => Some(Self::Loopback),
            HostReach::Private => Some(Self::Private),
            HostReach::LinkLocal => Some(Self::LinkLocal),
            HostReach::Reserved => Some(Self::Reserved),
            HostReach::Unspecified => Some(Self::Unspecified),
            HostReach::Public => None,
        }
    }
}

impl fmt::Display for RejectedReach {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Loopback => "a loopback address",
            Self::Private => "a private address",
            Self::LinkLocal => "a link-local address",
            Self::Reserved => "a reserved address",
            Self::Unspecified => "an unspecified address",
        })
    }
}

/// Why a page yielded no metadata. Shown as one line that never names a
/// URL, a host or an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebFailure {
    HostRejected(RejectedReach),
    Unresolved,
    TooManyRedirects,
    RedirectWithoutLocation,
    UnfollowableRedirect,
    Refused,
    TransportUnavailable,
    ConnectionFailed,
    TimedOut,
    Status(u16),
    NotHtml,
}

impl std::error::Error for WebFailure {}

impl fmt::Display for WebFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostRejected(reach) => write!(
                formatter,
                "E_WEB_HOST_REJECTED: the page's host is {reach}"
            ),
            Self::Unresolved => formatter.write_str(
                "E_WEB_HOST_UNRESOLVED: the page's host did not resolve",
            ),
            Self::TooManyRedirects => write!(
                formatter,
                "E_WEB_TOO_MANY_REDIRECTS: the page redirected more than \
                 {MAX_REDIRECTS} times"
            ),
            Self::RedirectWithoutLocation => formatter.write_str(
                "E_WEB_REDIRECT_WITHOUT_LOCATION: the page redirected without \
                 a Location",
            ),
            Self::UnfollowableRedirect => formatter.write_str(
                "E_WEB_REDIRECT_UNFOLLOWABLE: the page redirected to a URL \
                 that cannot be fetched",
            ),
            Self::Refused => formatter.write_str(
                "E_WEB_REFUSED: the page's address could not be fetched safely",
            ),
            Self::TransportUnavailable => formatter.write_str(
                "E_RESEARCH_TRANSPORT: the HTTP client could not be initialised",
            ),
            Self::ConnectionFailed => formatter.write_str(
                "E_WEB_CONNECTION_FAILED: the page could not be reached",
            ),
            Self::TimedOut => formatter.write_str(
                "E_WEB_TIMED_OUT: the page did not answer within the time \
                 allowed",
            ),
            Self::Status(status) => write!(
                formatter,
                "E_WEB_STATUS: the page answered HTTP {status}"
            ),
            Self::NotHtml => {
                formatter.write_str("E_WEB_NOT_HTML: the page is not HTML")
            }
        }
    }
}

/// How long the next lookup and the next request may each take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepBounds {
    pub lookup: Duration,
    pub request: Duration,
}

/// Each step's own bound, clipped to what the call has left, or `None` once
/// nothing is left.
///
/// Unlike the scholarly attempts, a step is not refused for lack of a whole
/// request budget: a web step has no retry to save the budget for, and a
/// clipped step that runs out ends the call as refusing it would.
pub fn step_bounds(deadline: &Deadline, now: Instant) -> Option<StepBounds> {
    let remaining = deadline.remaining(now);
    (!remaining.is_zero()).then(|| StepBounds {
        lookup: LOOKUP_CAP.min(remaining),
        request: deadline.per_request().min(remaining),
    })
}

/// The metadata `url`'s page declares, accessed on the call's UTC date.
///
/// # Errors
///
/// The [`WebFailure`] that ended the call before a page arrived.
pub fn fetch_page_metadata(
    url: &PageUrl,
    deadline: &Deadline,
    ports: &WebPorts<'_>,
) -> Result<SourceMetadata, WebFailure> {
    let accessed = PartialDate::utc_date_of(ports.clock.wall_now());
    let mut url = url.clone();
    for _ in 0..=MAX_REDIRECTS {
        let lookup = remaining_bounds(deadline, ports.clock)?.lookup;
        let addresses = admitted_addresses(&url, ports.resolver, lookup)?;
        let request = remaining_bounds(deadline, ports.clock)?.request;
        let response = ports.transport.get(&PageRequest {
            url: &url,
            addresses: &addresses,
            accept: PAGE_ACCEPT,
            max_body: MAX_PAGE_BODY,
            timeout: request,
        });
        match next_hop(response, &url)? {
            Hop::Page(body) => {
                return Ok(declared_metadata(
                    &ports.decoder.declarations(&body),
                    accessed,
                ))
            }
            Hop::Follow(next) => url = next,
        }
    }
    Err(WebFailure::TooManyRedirects)
}

fn remaining_bounds(
    deadline: &Deadline,
    clock: &dyn Clock,
) -> Result<StepBounds, WebFailure> {
    step_bounds(deadline, clock.now()).ok_or(WebFailure::TimedOut)
}

/// The addresses a request to `url` may connect to: the literal address, or
/// every address its name resolves to, each of which must be public. A name
/// is always resolved, `localhost` included, so its classification is never
/// the pre-resolution guess.
fn admitted_addresses(
    url: &PageUrl,
    resolver: &dyn HostResolver,
    within: Duration,
) -> Result<Vec<SocketAddr>, WebFailure> {
    let addresses = match url.host().address() {
        Some(address) => vec![SocketAddr::new(address, url.port())],
        None => resolver.resolve(url.host(), url.port(), within).map_err(
            |failure| match failure {
                ResolveFailure::Unresolved => WebFailure::Unresolved,
                ResolveFailure::Refused => WebFailure::Refused,
            },
        )?,
    };
    if addresses.is_empty() {
        return Err(WebFailure::Unresolved);
    }
    addresses
        .iter()
        .find_map(|address| RejectedReach::of(classify_address(address.ip())))
        .map_or(Ok(addresses), |rejected| {
            Err(WebFailure::HostRejected(rejected))
        })
}

enum Hop {
    Follow(PageUrl),
    Page(Vec<u8>),
}

fn next_hop(response: PageResponse, from: &PageUrl) -> Result<Hop, WebFailure> {
    let (status, location, content_type, body) = match response {
        PageResponse::Received {
            status,
            location,
            content_type,
            body,
        } => (status, location, content_type, body),
        PageResponse::Refused => return Err(WebFailure::Refused),
        PageResponse::Unavailable => {
            return Err(WebFailure::TransportUnavailable)
        }
        PageResponse::ConnectionFailed => {
            return Err(WebFailure::ConnectionFailed)
        }
        PageResponse::TimedOut => return Err(WebFailure::TimedOut),
    };
    match status {
        300..=399 => {
            let location =
                location.ok_or(WebFailure::RedirectWithoutLocation)?;
            from.follow(&location)
                .map(Hop::Follow)
                .map_err(|_| WebFailure::UnfollowableRedirect)
        }
        200..=299 if content_type.as_deref().is_some_and(is_html) => {
            Ok(Hop::Page(body))
        }
        200..=299 => Err(WebFailure::NotHtml),
        other => Err(WebFailure::Status(other)),
    }
}

fn is_html(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or_default().trim();
    HTML_MEDIA_TYPES
        .iter()
        .any(|html| media_type.eq_ignore_ascii_case(html))
}
