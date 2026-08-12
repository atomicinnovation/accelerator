//! How reachable the address a host names is.
//!
//! Three limits are taken positions rather than oversights.
//!
//! The check is **pre-resolution only**: a public hostname resolving to
//! `169.254.169.254` still classifies as [`HostReach::Public`], and nothing
//! re-checks after DNS.
//!
//! It covers **only the initial location**. The daemon's `navigate` command
//! takes an arbitrary URL per request and follows it with no classification at
//! all, and the `links` command hands the agent a crawlable set whose
//! same-origin flag drives route following. This module is the front door, not
//! a boundary around the navigation surface.
//!
//! It says nothing about **path locations**: a repository path is a valid
//! source location wherever it points, and nothing confirms it lies inside a
//! repository.

use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use crate::host::Host;

/// The classification a host's address carries, in the vocabulary the
/// rejection message speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostReach {
    /// The local machine talking to itself. Always allowed: it carries
    /// neither the internal-network nor the plaintext-interception risk the
    /// two flags guard against.
    Loopback,
    /// RFC 1918 and IPv6 unique-local.
    Private,
    /// `169.254.0.0/16` and `fe80::/10` — the cloud metadata endpoint's home.
    LinkLocal,
    /// Not internet-routable to a normal destination: carrier-grade NAT,
    /// benchmarking, multicast and the rest of the reserved space.
    Reserved,
    /// `0.0.0.0` and `::` — no host at all, so no flag recovers it.
    Unspecified,
    Public,
}

impl HostReach {
    /// The word the rejection message uses.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Loopback => "loopback",
            Self::Private => "RFC1918",
            Self::LinkLocal => "link-local",
            Self::Reserved => "reserved",
            Self::Unspecified => "wildcard",
            Self::Public => "public",
        }
    }
}

/// The one hostname classified without resolving it.
///
/// The shell's carve-out names `localhost` alongside `127.0.0.1`, and it is
/// the skill's primary documented invocation. Resolving it would be a DNS
/// lookup this module deliberately does not make, and treating it as public
/// would reject `http://localhost:3000` — so the name is honoured as written.
/// Exactly the name, as the shell matched it: a `foo.localhost` subdomain
/// resolves wherever its zone says.
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
fn classify_address(address: IpAddr) -> HostReach {
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

#[cfg(test)]
mod tests {
    use super::classify;
    use super::HostReach;
    use crate::host::Host;
    use crate::host::HostError;

    /// A host that fails canonicalisation classifies as nothing, so an
    /// unparseable fixture is reported as such rather than panicking.
    fn reach(authority: &str) -> Result<HostReach, HostError> {
        Host::canonicalise(authority).map(|host| classify(&host))
    }

    #[test]
    fn the_shell_s_own_classifications_survive() -> Result<(), HostError> {
        assert_eq!(reach("127.0.0.1")?, HostReach::Loopback);
        assert_eq!(reach("[::1]")?, HostReach::Loopback);
        assert_eq!(reach("[::ffff:127.0.0.1]")?, HostReach::Loopback);
        assert_eq!(reach("0.0.0.0")?, HostReach::Unspecified);
        assert_eq!(reach("[::]")?, HostReach::Unspecified);
        assert_eq!(reach("10.0.0.1")?, HostReach::Private);
        assert_eq!(reach("192.168.1.1")?, HostReach::Private);
        assert_eq!(reach("169.254.169.254")?, HostReach::LinkLocal);
        assert_eq!(reach("[fe80::1]")?, HostReach::LinkLocal);
        assert_eq!(reach("example.com")?, HostReach::Public);
        assert_eq!(reach("93.184.216.34")?, HostReach::Public);
        Ok(())
    }

    #[test]
    fn the_rfc1918_boundary_holds_at_both_edges_and_just_outside(
    ) -> Result<(), HostError> {
        assert_eq!(reach("172.16.0.0")?, HostReach::Private);
        assert_eq!(reach("172.31.255.255")?, HostReach::Private);
        assert_eq!(reach("172.15.255.255")?, HostReach::Public);
        assert_eq!(reach("172.32.0.0")?, HostReach::Public);
        Ok(())
    }

    /// The encodings `classify_internal`'s regexes never matched. Each is a
    /// route to a link-local metadata endpoint or an internal host that the
    /// shell would have classified public.
    #[test]
    fn every_newly_rejected_encoding_classifies_internally(
    ) -> Result<(), HostError> {
        for (authority, expected) in [
            ("[::ffff:169.254.169.254]", HostReach::LinkLocal),
            ("[::ffff:10.0.0.1]", HostReach::Private),
            ("[fd00::1]", HostReach::Private),
            ("100.64.0.1", HostReach::Reserved),
            ("0.1.2.3", HostReach::Reserved),
            ("[2002:a9fe:a9fe::]", HostReach::LinkLocal),
            // Teredo carries 169.254.169.254 bitwise-inverted: !0xa9fe = 0x5601.
            ("[2001:0:0:0:0:0:5601:5601]", HostReach::LinkLocal),
            ("[64:ff9b::a9fe:a9fe]", HostReach::LinkLocal),
            ("192.0.0.1", HostReach::Reserved),
            ("198.18.0.1", HostReach::Reserved),
            ("240.0.0.1", HostReach::Reserved),
            ("224.0.0.1", HostReach::Reserved),
        ] {
            assert_eq!(reach(authority)?, expected, "{authority}");
        }
        Ok(())
    }

    /// `::1` fully expanded is the same address, so it is loopback rather than
    /// a newly-rejected encoding — the widening from the shell's two literal
    /// strings to every address `is_loopback` holds for.
    #[test]
    fn the_widened_loopback_set_covers_the_expanded_and_ranged_forms(
    ) -> Result<(), HostError> {
        for authority in
            ["[0:0:0:0:0:0:0:1]", "127.0.0.2", "127.1.2.3", "[::1]"]
        {
            assert_eq!(reach(authority)?, HostReach::Loopback, "{authority}");
        }
        Ok(())
    }

    /// The shell's other literal carve-out. It is a name, so no address
    /// predicate reaches it.
    #[test]
    fn the_loopback_name_is_honoured_without_being_resolved(
    ) -> Result<(), HostError> {
        assert_eq!(reach("localhost")?, HostReach::Loopback);
        assert_eq!(reach("localhost:3000")?, HostReach::Loopback);
        assert_eq!(reach("LOCALHOST")?, HostReach::Loopback);
        assert_eq!(reach("localhost.")?, HostReach::Loopback);
        assert_eq!(reach("evil.localhost")?, HostReach::Public);
        assert_eq!(reach("localhost.evil.com")?, HostReach::Public);
        Ok(())
    }

    #[test]
    fn the_ipv4_compatible_form_unwraps_the_same_way_the_mapped_one_does(
    ) -> Result<(), HostError> {
        assert_eq!(reach("[::169.254.169.254]")?, HostReach::LinkLocal);
        assert_eq!(reach("[::10.0.0.1]")?, HostReach::Private);
        Ok(())
    }

    #[test]
    fn every_variant_names_itself_for_the_rejection_message() {
        assert_eq!(HostReach::Loopback.description(), "loopback");
        assert_eq!(HostReach::Private.description(), "RFC1918");
        assert_eq!(HostReach::LinkLocal.description(), "link-local");
        assert_eq!(HostReach::Reserved.description(), "reserved");
        assert_eq!(HostReach::Unspecified.description(), "wildcard");
    }
}
