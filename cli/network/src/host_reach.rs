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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostReach {
    Loopback,
    /// RFC 1918, and IPv6 unique-local and site-local.
    Private,
    /// `169.254.0.0/16` and `fe80::/10` — the cloud metadata endpoint's home.
    LinkLocal,
    /// Not internet-routable to a normal destination: carrier-grade NAT,
    /// benchmarking, documentation, multicast and the rest of the reserved
    /// space.
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
        || (first == 192 && second == 0 && address.octets()[2] == 2)
        || (first == 198 && (18..20).contains(&second))
        || (first == 198 && second == 51 && address.octets()[2] == 100)
        || (first == 203 && second == 0 && address.octets()[2] == 113)
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
    if is_unique_local(address) || is_site_local(address) {
        HostReach::Private
    } else if is_v6_link_local(address) {
        HostReach::LinkLocal
    } else if address.is_multicast() || is_v6_reserved(address) {
        HostReach::Reserved
    } else {
        HostReach::Public
    }
}

/// The IPv4 address a transition encoding carries, if any.
///
/// `to_ipv4` covers the IPv4-mapped `::ffff:a.b.c.d` and IPv4-compatible
/// `::a.b.c.d` forms alike with one unwrap; 6to4, Teredo, NAT64 and the
/// IPv4-translated `::ffff:0:a.b.c.d` each carry theirs in a different
/// position.
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
    if segments[..4].iter().all(|segment| *segment == 0)
        && segments[4] == 0xffff
        && segments[5] == 0
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

/// Deprecated by RFC 3879, but a network that still routes it routes it
/// internally.
const fn is_site_local(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xffc0 == 0xfec0
}

/// Local-use NAT64 (`64:ff9b:1::/48`), whose embedded address sits where the
/// operator's prefix length puts it, is refused whole rather than unwrapped;
/// discard-only `100::/64` and documentation `2001:db8::/32` and `3fff::/20`
/// are never anyone's page.
fn is_v6_reserved(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    (segments[0] == 0x0064 && segments[1] == 0xff9b && segments[2] == 0x0001)
        || (segments[0] == 0x0100
            && segments[1..4].iter().all(|segment| *segment == 0))
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] == 0x3fff && segments[1] & 0xf000 == 0)
}
