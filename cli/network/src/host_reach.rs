//! How reachable the address a host names is.
//!
//! For a hostname the classification is **pre-resolution**: a public hostname
//! resolving to `169.254.169.254` still classifies as [`HostReach::Public`]. A
//! caller that resolves names must classify each address it resolved to with
//! [`classify_address`] instead.

use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use crate::host::Host;

/// The classification a host's address carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostReach {
    /// The local machine talking to itself.
    Loopback,
    /// RFC 1918 and IPv6 unique-local.
    Private,
    /// `169.254.0.0/16` and `fe80::/10` — the cloud metadata endpoint's home.
    LinkLocal,
    /// Not internet-routable to a normal destination: carrier-grade NAT,
    /// benchmarking, multicast and the rest of the reserved space.
    Reserved,
    /// `0.0.0.0` and `::` — no host at all.
    Unspecified,
    Public,
}

/// The one hostname classified without resolving it.
///
/// A pre-resolution convenience: a caller that classifies without resolving
/// would otherwise call `localhost` public. A caller that resolves the name
/// classifies the addresses it resolves to and has no need of it. Exactly the
/// name: a `foo.localhost` subdomain resolves wherever its zone says.
const LOOPBACK_NAME: &str = "localhost";

/// Classifies the address `host` names; any other hostname is
/// [`HostReach::Public`], since nothing is resolved here.
#[must_use]
pub fn classify(host: &Host) -> HostReach {
    match host.address() {
        Some(address) => classify_address(address),
        None if host.to_string() == LOOPBACK_NAME => HostReach::Loopback,
        None => HostReach::Public,
    }
}

/// Classifies an address directly, as a caller does for each address a name
/// resolved to.
#[must_use]
pub fn classify_address(address: IpAddr) -> HostReach {
    match address {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => classify_v6(v6),
    }
}

fn classify_v4(address: Ipv4Addr) -> HostReach {
    let [first, second, ..] = address.octets();
    if address.is_unspecified() {
        HostReach::Unspecified
    } else if address.is_loopback() {
        HostReach::Loopback
    } else if address.is_private() {
        HostReach::Private
    } else if address.is_link_local() {
        HostReach::LinkLocal
    } else if address.is_multicast()
        || first == 0
        || (first == 100 && (64..128).contains(&second))
        || (first == 192 && second == 0 && address.octets()[2] == 0)
        || (first == 198 && (18..20).contains(&second))
        || first >= 240
    {
        HostReach::Reserved
    } else {
        HostReach::Public
    }
}

/// The transition encodings that embed an IPv4 address are unwrapped and
/// re-classified on it, so `2002:a9fe:a9fe::` cannot reach the metadata
/// endpoint through a form the address predicates never contemplated.
fn classify_v6(address: Ipv6Addr) -> HostReach {
    // `::` and `::1` are themselves IPv4-compatible forms as far as `to_ipv4`
    // is concerned, so they are answered before anything is unwrapped.
    if address.is_unspecified() {
        return HostReach::Unspecified;
    }
    if address.is_loopback() {
        return HostReach::Loopback;
    }
    if let Some(embedded) = embedded_v4(address) {
        return classify_v4(embedded);
    }
    if is_unique_local(address) {
        HostReach::Private
    } else if is_v6_link_local(address) {
        HostReach::LinkLocal
    } else if address.is_multicast() {
        HostReach::Reserved
    } else {
        HostReach::Public
    }
}

/// The IPv4 address a transition encoding carries, if any.
///
/// `to_ipv4` covers the IPv4-mapped `::ffff:a.b.c.d` and IPv4-compatible
/// `::a.b.c.d` forms alike with one unwrap; 6to4, Teredo and NAT64 each carry
/// theirs in a different position.
fn embedded_v4(address: Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = address.segments();
    if segments[0] == 0x2002 {
        return Some(Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            (segments[1] & 0xff) as u8,
            (segments[2] >> 8) as u8,
            (segments[2] & 0xff) as u8,
        ));
    }
    if segments[0] == 0x2001 && segments[1] == 0 {
        // RFC 4380 stores Teredo's mapped IPv4 address bitwise-inverted.
        let mapped = (u32::from(segments[6]) << 16) | u32::from(segments[7]);
        return Some(Ipv4Addr::from(!mapped));
    }
    if segments[0] == 0x0064
        && segments[1] == 0xff9b
        && segments[2..6].iter().all(|segment| *segment == 0)
    {
        return Some(Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        ));
    }
    address.to_ipv4()
}

const fn is_unique_local(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xfe00 == 0xfc00
}

const fn is_v6_link_local(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xffc0 == 0xfe80
}
