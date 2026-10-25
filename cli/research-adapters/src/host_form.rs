//! The form of an admitted host the HTTP client connects to, read by the
//! same parser the client uses so the two can never disagree about it.

use std::net::IpAddr;

use network::Host;
use reqwest::Url;

/// A name's ASCII form, IDNA-converted as reqwest converts it, or an
/// address's own text.
///
/// `None` for a name the URL parser refuses, and for one it reads as an
/// address: under UTS 46 a fullwidth `１２７.０.０.１` folds into
/// `127.0.0.1`, which admission classified as a name and never as the
/// address it becomes.
pub fn ascii_host(host: &Host) -> Option<String> {
    if let Some(address) = host.address() {
        return Some(address.to_string());
    }
    match url::Host::parse(&host.to_string()).ok()? {
        url::Host::Domain(domain) => Some(domain),
        url::Host::Ipv4(_) | url::Host::Ipv6(_) => None,
    }
}

/// Whether the host `url` names is exactly the one admitted: the same ASCII
/// name, or the same address.
pub fn admitted_host_matches(url: &Url, admitted: &Host) -> bool {
    match (url.host(), admitted.address()) {
        (Some(url::Host::Domain(domain)), None) => {
            ascii_host(admitted).is_some_and(|ascii| ascii == domain)
        }
        (Some(url::Host::Ipv4(read)), Some(IpAddr::V4(admitted))) => {
            read == admitted
        }
        (Some(url::Host::Ipv6(read)), Some(IpAddr::V6(admitted))) => {
            read == admitted
        }
        _ => false,
    }
}
