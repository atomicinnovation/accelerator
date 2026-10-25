//! A URL authority read for connecting: its host and the port it names.

use std::fmt;

use crate::host::decode;
use crate::host::split_authority;
use crate::host::Host;
use crate::host::HostError;

/// Why an authority could not be read for connecting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    Host(HostError),
    /// The port is not a decimal number from 1 to 65535.
    Port,
}

impl std::error::Error for AuthorityError {}

impl fmt::Display for AuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => write!(formatter, "{error}"),
            Self::Port => {
                formatter.write_str("the port is not a number from 1 to 65535")
            }
        }
    }
}

/// A canonical host and the port written beside it.
///
/// Unlike [`Host::canonicalise`], which ignores the port text because its
/// callers never connect to it, a malformed port is refused here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    host: Host,
    port: Option<u16>,
}

impl Authority {
    /// Reads a raw URL authority (`host`, `host:port`, `[v6]:port`).
    ///
    /// # Errors
    ///
    /// [`AuthorityError::Host`] when the host cannot be canonicalised, and
    /// [`AuthorityError::Port`] when a written port is out of range or not a
    /// number.
    pub fn parse(raw: &str) -> Result<Self, AuthorityError> {
        let decoded = decode(raw).map_err(AuthorityError::Host)?;
        let (host, remainder) = split_authority(&decoded);
        let host = Host::from_split(host).map_err(AuthorityError::Host)?;
        let port = port_of(remainder)?;
        Ok(Self { host, port })
    }

    #[must_use]
    pub const fn host(&self) -> &Host {
        &self.host
    }

    /// The written port, or `None` when the scheme's default applies: no
    /// port, or an empty one (RFC 3986 §3.2.3).
    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }
}

fn port_of(remainder: &str) -> Result<Option<u16>, AuthorityError> {
    if remainder.is_empty() {
        return Ok(None);
    }
    let digits = remainder.strip_prefix(':').ok_or(AuthorityError::Port)?;
    if digits.is_empty() {
        return Ok(None);
    }
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AuthorityError::Port);
    }
    digits
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .map(Some)
        .ok_or(AuthorityError::Port)
}
