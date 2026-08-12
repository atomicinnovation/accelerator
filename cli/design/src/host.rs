//! The canonical host a URL names, and the rejections canonicalisation makes.

use std::fmt;
use std::net::IpAddr;
use std::str::FromStr as _;

/// Why an authority string could not become a [`Host`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// A userinfo segment (`user@host`, `user:pass@host`) — a suffix-confusion
    /// vector, rejected before anything else is read.
    Userinfo,
    /// The authority named no host at all.
    Empty,
    /// A control character survived percent-decoding.
    ControlCharacter,
    /// The host looks numeric but is not a strictly-parseable address, so it
    /// is rejected rather than falling through to hostname treatment: a
    /// decimal, hexadecimal or zero-padded octal IPv4 encoding, or an
    /// over-long dotted form.
    NumericEncoding,
}

impl std::error::Error for HostError {}

impl HostError {
    /// The rejection as the caller reads it. Only the numeric-encoding case
    /// names the authority, matching the shell: the others describe a property
    /// of the URL rather than of a host it successfully identified.
    #[must_use]
    pub fn message(&self, authority: &str) -> String {
        match self {
            Self::Userinfo => "URL contains a userinfo segment (user@host), \
                               which is not permitted."
                .to_owned(),
            Self::Empty => "URL names no host.".to_owned(),
            Self::ControlCharacter => format!(
                "host '{authority}' contains a control character, which is \
                 not permitted."
            ),
            Self::NumericEncoding => format!(
                "host '{authority}' uses a numeric IPv4 encoding (decimal, \
                 hex, or octal), which is not permitted."
            ),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message("<host>"))
    }
}

/// A canonicalised host: lowercased, with any port, IPv6 brackets, IPv6
/// zone-id and single trailing dot removed.
///
/// Construction is the only way to obtain one, so every consumer is working
/// from the same canonical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    canonical: String,
    address: Option<IpAddr>,
}

impl Host {
    /// Canonicalises a raw URL authority (`host`, `host:port`, `[v6]:port`).
    ///
    /// # Errors
    ///
    /// A [`HostError`] naming which canonicalisation rule the authority broke.
    pub fn canonicalise(authority: &str) -> Result<Self, HostError> {
        if authority.contains('@') {
            return Err(HostError::Userinfo);
        }

        let decoded = percent_decode(authority);
        if decoded.chars().any(char::is_control) {
            return Err(HostError::ControlCharacter);
        }

        let lowered = decoded.to_lowercase();
        let stripped = strip_port_and_brackets(&lowered);
        let canonical = stripped.strip_suffix('.').unwrap_or(stripped);

        if canonical.is_empty() {
            return Err(HostError::Empty);
        }

        let address = IpAddr::from_str(canonical).ok();
        if address.is_none() && looks_numeric(canonical) {
            return Err(HostError::NumericEncoding);
        }

        Ok(Self {
            canonical: canonical.to_owned(),
            address,
        })
    }

    /// The address this host names, or `None` when it is a hostname.
    #[must_use]
    pub const fn address(&self) -> Option<IpAddr> {
        self.address
    }
}

impl fmt::Display for Host {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.canonical)
    }
}

/// A bracketed authority is IPv6: the brackets and any zone-id go, and the
/// colons inside are part of the address. An unbracketed one keeps everything
/// before its first colon, which is the port separator.
fn strip_port_and_brackets(authority: &str) -> &str {
    let Some(rest) = authority.strip_prefix('[') else {
        return authority.split(':').next().unwrap_or(authority);
    };
    let inside = rest.split(']').next().unwrap_or(rest);
    inside.split('%').next().unwrap_or(inside)
}

/// Whether a host that failed strict address parsing must nonetheless be
/// rejected rather than treated as a hostname.
///
/// The predicate is over the whole host, not per label: `10.0.0.1.example.com`
/// is a hostname whose first label merely looks numeric, while `1.2.3.4.5` and
/// `0x7f000001` are numeric encodings of an address.
fn looks_numeric(host: &str) -> bool {
    host.contains(':') || host.split('.').all(is_numeric_label)
}

/// A label written as an address component: decimal digits, optionally
/// zero-padded, or an explicit `0x` hexadecimal escape. A bare hex-looking
/// word (`cafe`) is a hostname label, not a numeric one.
fn is_numeric_label(label: &str) -> bool {
    label.strip_prefix("0x").map_or_else(
        || !label.is_empty() && label.bytes().all(|byte| byte.is_ascii_digit()),
        |hex| {
            !hex.is_empty() && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        },
    )
}

/// Decodes `%XX` escapes, leaving a malformed escape as written so it reaches
/// the control-character and numeric checks as the literal text it is.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = bytes
            .get(index + 1..index + 3)
            .filter(|_| bytes[index] == b'%')
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(byte) = escape {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::str::FromStr as _;

    use super::Host;
    use super::HostError;

    type TestError = Box<dyn std::error::Error>;

    fn canonical(authority: &str) -> Result<String, TestError> {
        Ok(Host::canonicalise(authority)?.to_string())
    }

    #[test]
    fn a_port_is_stripped() -> Result<(), TestError> {
        assert_eq!(canonical("example.com:3000")?, "example.com");
        assert_eq!(canonical("localhost:8080")?, "localhost");
        Ok(())
    }

    #[test]
    fn the_host_is_lowercased() -> Result<(), TestError> {
        assert_eq!(canonical("EXAMPLE.COM")?, "example.com");
        Ok(())
    }

    #[test]
    fn a_single_trailing_dot_is_stripped() -> Result<(), TestError> {
        assert_eq!(canonical("example.com.")?, "example.com");
        Ok(())
    }

    #[test]
    fn brackets_and_a_zone_id_are_stripped() -> Result<(), TestError> {
        assert_eq!(canonical("[::1]:3000")?, "::1");
        assert_eq!(canonical("[fe80::1%eth0]")?, "fe80::1");
        Ok(())
    }

    #[test]
    fn a_userinfo_segment_is_rejected() {
        assert_eq!(
            Host::canonicalise("user:pass@127.0.0.1@evil.com"),
            Err(HostError::Userinfo)
        );
        assert_eq!(
            Host::canonicalise("user@example.com"),
            Err(HostError::Userinfo)
        );
    }

    #[test]
    fn an_empty_authority_is_rejected() {
        assert_eq!(Host::canonicalise(""), Err(HostError::Empty));
        assert_eq!(Host::canonicalise(":3000"), Err(HostError::Empty));
    }

    #[test]
    fn every_numeric_encoding_that_is_not_an_address_is_rejected() {
        for authority in [
            "2130706433",
            "0x7f000001",
            "0177.0.0.1",
            "1.2.3.4.5",
            "0x7f.1",
            "127.0.0.01",
            "10.0.0.256",
        ] {
            assert_eq!(
                Host::canonicalise(authority),
                Err(HostError::NumericEncoding),
                "{authority} must not fall through to hostname treatment"
            );
        }
    }

    #[test]
    fn a_hostname_whose_first_label_looks_numeric_is_still_a_hostname(
    ) -> Result<(), TestError> {
        let host = Host::canonicalise("10.0.0.1.example.com")?;
        assert_eq!(host.to_string(), "10.0.0.1.example.com");
        assert_eq!(host.address(), None);
        Ok(())
    }

    #[test]
    fn a_control_character_is_rejected_rather_than_carried() {
        assert_eq!(
            Host::canonicalise("127.0.0.1%0d%0aevil"),
            Err(HostError::ControlCharacter)
        );
    }

    #[test]
    fn a_percent_escaped_address_decodes_before_classification(
    ) -> Result<(), TestError> {
        let host = Host::canonicalise("%31%32%37.0.0.1")?;
        assert_eq!(host.address(), Some(IpAddr::from_str("127.0.0.1")?));
        Ok(())
    }

    #[test]
    fn an_address_is_carried_alongside_its_canonical_text(
    ) -> Result<(), TestError> {
        let host = Host::canonicalise("[::FFFF:127.0.0.1]")?;
        assert!(host.address().is_some());
        Ok(())
    }
}
