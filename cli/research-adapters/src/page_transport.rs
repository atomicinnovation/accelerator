//! The blocking HTTP transport a web page is fetched through, connecting
//! only to the addresses admission vetted.
//!
//! Pinning fails closed: the client's only resolver answers the admitted name
//! with the admitted addresses and refuses every other, proxies and redirects
//! are off, and a URL whose host the client reads differently from the one
//! admitted is never sent. An address literal consults no resolver, so that
//! last check is all that pins one.

use std::error::Error;
use std::fmt;
use std::io::Read as _;
use std::net::SocketAddr;
use std::sync::Arc;

use reqwest::blocking::Client;
use reqwest::dns::Addrs;
use reqwest::dns::Name;
use reqwest::dns::Resolve;
use reqwest::dns::Resolving;
use reqwest::header::HeaderMap;
use reqwest::redirect;
use reqwest::Url;
use research::sources::request::USER_AGENT;
use research::sources::web::fetch::PageRequest;
use research::sources::web::fetch::PageResponse;
use research::sources::web::fetch::PageTransport;

use crate::host_form::admitted_host_matches;
use crate::host_form::ascii_host;
use crate::transport::install_crypto_provider;
use crate::transport::is_timeout;

pub struct HttpPageTransport;

impl PageTransport for HttpPageTransport {
    fn get(&self, request: &PageRequest<'_>) -> PageResponse {
        let Ok(url) = Url::parse(&request.url.to_string()) else {
            return PageResponse::Refused;
        };
        let admitted = request.url.host();
        let Some(name) = ascii_host(admitted) else {
            return PageResponse::Refused;
        };
        if !admitted_host_matches(&url, admitted) {
            return PageResponse::Refused;
        }
        install_crypto_provider();
        let client = Client::builder()
            .redirect(redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .no_proxy()
            .timeout(request.timeout)
            .dns_resolver(Arc::new(PinnedResolver::new(
                &name,
                request.addresses.to_vec(),
            )))
            .build();
        let Ok(client) = client else {
            return PageResponse::ConnectionFailed;
        };
        let sent = client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", request.accept)
            .send();
        match sent {
            Ok(response) => receive(response, request.max_body),
            Err(error) if error.is_timeout() => PageResponse::TimedOut,
            Err(_) => PageResponse::ConnectionFailed,
        }
    }
}

/// Bytes past `max_body` are dropped rather than the whole page refused.
fn receive(
    response: reqwest::blocking::Response,
    max_body: usize,
) -> PageResponse {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let Ok(location) = utf8_header(&headers, "location") else {
        return PageResponse::Refused;
    };
    let mut body = Vec::new();
    let read = response
        .take(u64::try_from(max_body).unwrap_or(u64::MAX))
        .read_to_end(&mut body);
    match read {
        Ok(_) => PageResponse::Received {
            status,
            location,
            content_type: utf8_header(&headers, "content-type").ok().flatten(),
            body,
        },
        Err(error) if is_timeout(&error) => PageResponse::TimedOut,
        Err(_) => PageResponse::ConnectionFailed,
    }
}

/// The header's raw bytes as UTF-8, so a raw UTF-8 `Location` reaches the
/// domain to judge rather than being lost as non-visible ASCII.
fn utf8_header(
    headers: &HeaderMap,
    name: &str,
) -> Result<Option<String>, std::str::Utf8Error> {
    headers
        .get(name)
        .map(|value| std::str::from_utf8(value.as_bytes()).map(str::to_owned))
        .transpose()
}

/// Answers its one name with its admitted addresses and refuses any other,
/// leaving the client no lookup of its own to fall back to.
pub struct PinnedResolver {
    name: String,
    addresses: Vec<SocketAddr>,
}

impl PinnedResolver {
    pub fn new(name: &str, addresses: Vec<SocketAddr>) -> Self {
        Self {
            name: name.to_owned(),
            addresses,
        }
    }
}

impl Resolve for PinnedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let answer =
            (name.as_str() == self.name).then(|| self.addresses.clone());
        Box::pin(async move {
            answer
                .map(|addresses| Box::new(addresses.into_iter()) as Addrs)
                .ok_or_else(|| {
                    Box::new(UnpinnedName) as Box<dyn Error + Send + Sync>
                })
        })
    }
}

#[derive(Debug)]
struct UnpinnedName;

impl fmt::Display for UnpinnedName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the name was not admitted")
    }
}

impl Error for UnpinnedName {}
