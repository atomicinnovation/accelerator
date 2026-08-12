//! Which commands may be forwarded to the Node runner.
//!
//! Not exposing a command on the Rust surface is not the same as making it
//! unreachable: arguments are forwarded verbatim, and the runner dispatches on
//! the first of them. A stray `executor daemon` would start a second
//! foreground daemon that binds a fresh port, overwrites the live one's state
//! files, orphans it mid-crawl and never returns — the failure the launcher's
//! kill-on-timeout exists to prevent.
//!
//! So forwarding is an allowlist. A command added to the runner is unreachable
//! until it is named here, which is the safe direction to fail in.

use std::fmt;

/// Every command a caller may forward.
pub const FORWARDABLE_COMMANDS: [&str; 7] = [
    "ping",
    "navigate",
    "snapshot",
    "screenshot",
    "evaluate",
    "links",
    "daemon-stop",
];

/// The commands the runner dispatches internally, which a caller must never
/// reach. Named rather than merely omitted, so the rejection can say why.
const INTERNAL_COMMANDS: [&str; 1] = ["daemon"];

/// Why a command was not forwarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotForwardable {
    /// An internal runner subcommand, reachable only by the launcher itself.
    Internal(String),
    Unknown(String),
}

impl std::error::Error for NotForwardable {}

impl fmt::Display for NotForwardable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(command) => write!(
                formatter,
                "'{command}' is an internal executor command and cannot be \
                 invoked directly. The launcher starts and stops the daemon \
                 itself."
            ),
            Self::Unknown(command) => write!(
                formatter,
                "unknown executor command '{command}'. Valid commands: {}",
                FORWARDABLE_COMMANDS.join(", ")
            ),
        }
    }
}

/// Whether `command` may be forwarded.
///
/// # Errors
///
/// A [`NotForwardable`] naming the command, distinguishing an internal
/// subcommand from an unrecognised one.
pub fn check(command: &str) -> Result<(), NotForwardable> {
    if FORWARDABLE_COMMANDS.contains(&command) {
        return Ok(());
    }
    if INTERNAL_COMMANDS.contains(&command) {
        return Err(NotForwardable::Internal(command.to_owned()));
    }
    Err(NotForwardable::Unknown(command.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::check;
    use super::NotForwardable;
    use super::FORWARDABLE_COMMANDS;

    #[test]
    fn every_forwardable_command_is_accepted() {
        for command in FORWARDABLE_COMMANDS {
            assert_eq!(check(command), Ok(()), "{command}");
        }
    }

    /// The one that would start a second daemon.
    #[test]
    fn the_internal_daemon_command_is_rejected_as_internal(
    ) -> Result<(), String> {
        let Err(error) = check("daemon") else {
            return Err("expected a rejection".to_owned());
        };
        assert_eq!(error, NotForwardable::Internal("daemon".to_owned()));
        assert!(error.to_string().contains("internal executor command"));
        Ok(())
    }

    #[test]
    fn an_unrecognised_command_lists_what_is_valid() -> Result<(), String> {
        let Err(error) = check("rm-rf") else {
            return Err("expected a rejection".to_owned());
        };
        assert_eq!(error, NotForwardable::Unknown("rm-rf".to_owned()));
        for command in FORWARDABLE_COMMANDS {
            assert!(error.to_string().contains(command), "{command}");
        }
        Ok(())
    }

    #[test]
    fn the_allowlist_is_matched_exactly_not_by_prefix() {
        for command in ["ping ", " ping", "PING", "daemon-stopped", "nav"] {
            assert!(check(command).is_err(), "{command}");
        }
    }
}
