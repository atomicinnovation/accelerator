//! The identity record the launcher hands the daemon over an inherited pipe.
//!
//! One observer, one writer, one atomic write. The launcher learns the pid the
//! moment `spawn` returns — too late to pass it through the environment, since
//! a child's `envp` is fixed at `exec` and built before the fork — so the
//! values travel down a pipe the child inherits instead.
//!
//! The daemon reads this to end of input and parses it *before* publishing
//! anything or opening its listening socket. That ordering is what stops a
//! live daemon ever having a partial identity record: readiness is the record
//! appearing, so a writer that could be interrupted between the two would
//! leave a daemon whose record has no start time — which the reuse verdict
//! reads as stale, deletes, and respawns around, while the first daemon still
//! holds a port, a browser and the crawl's page state.
//!
//! Four newline-delimited fields in a fixed order. A short or malformed read
//! is a daemon-side failure, not a default.

use std::fmt;

use crate::executor::daemon_identity::RecordedStartTime;

/// The values handed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub pid: i32,
    pub start_time: RecordedStartTime,
    /// A CSPRNG value the daemon requires on its first accepted connection.
    pub token: String,
}

/// Why a record could not be read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedIdentity {
    /// Fewer than four fields — the launcher died mid-write, or never wrote.
    Truncated {
        fields: usize,
    },
    UnparseablePid(String),
    UnparseableStartTime(String),
    UnknownStartTimeSource(String),
    EmptyToken,
}

impl std::error::Error for MalformedIdentity {}

impl fmt::Display for MalformedIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { fields } => write!(
                formatter,
                "identity handoff carried {fields} of 4 fields; the launcher \
                 did not complete its write"
            ),
            Self::UnparseablePid(raw) => {
                write!(formatter, "identity pid {raw:?} is not an integer")
            }
            Self::UnparseableStartTime(raw) => write!(
                formatter,
                "identity start time {raw:?} is not an integer"
            ),
            Self::UnknownStartTimeSource(raw) => write!(
                formatter,
                "identity start-time source {raw:?} is not one of p, w, u"
            ),
            Self::EmptyToken => {
                write!(formatter, "identity carried an empty token")
            }
        }
    }
}

/// The single tag byte naming where the start time came from.
const PROBE: &str = "p";
const WALLCLOCK: &str = "w";
const WRITER_UNAVAILABLE: &str = "u";

impl Identity {
    /// The wire form: `pid\nstart_time\nsource\ntoken\n`.
    ///
    /// `WriterUnavailable` still writes a start-time field, so the field count
    /// is fixed and a truncated read is unambiguous.
    #[must_use]
    pub fn render(&self) -> String {
        let (seconds, source) = match self.start_time {
            RecordedStartTime::Probe(seconds) => (seconds, PROBE),
            RecordedStartTime::Wallclock(seconds) => (seconds, WALLCLOCK),
            RecordedStartTime::WriterUnavailable
            | RecordedStartTime::AbsentOrUnparseable => (0, WRITER_UNAVAILABLE),
        };
        format!("{}\n{seconds}\n{source}\n{}\n", self.pid, self.token)
    }

    /// Parses what the daemon read off the pipe.
    ///
    /// # Errors
    ///
    /// A [`MalformedIdentity`] naming which field failed. Every variant is a
    /// reason for the daemon to log and exit before creating a browser or
    /// binding a socket, rather than proceeding on defaults.
    pub fn parse(raw: &str) -> Result<Self, MalformedIdentity> {
        // Exactly one trailing newline, not every one: trimming greedily
        // would make an empty token indistinguishable from a field that never
        // arrived, collapsing two different failures into one.
        let body = raw.strip_suffix('\n').unwrap_or(raw);
        let fields: Vec<&str> = body.split('\n').collect();
        let [pid, seconds, source, token] = fields.as_slice() else {
            return Err(MalformedIdentity::Truncated {
                fields: if raw.is_empty() { 0 } else { fields.len() },
            });
        };

        let pid: i32 = pid.parse().map_err(|_| {
            MalformedIdentity::UnparseablePid((*pid).to_owned())
        })?;
        let parse_seconds = || {
            seconds.parse::<u64>().map_err(|_| {
                MalformedIdentity::UnparseableStartTime((*seconds).to_owned())
            })
        };
        let start_time = match *source {
            PROBE => RecordedStartTime::Probe(parse_seconds()?),
            WALLCLOCK => RecordedStartTime::Wallclock(parse_seconds()?),
            WRITER_UNAVAILABLE => RecordedStartTime::WriterUnavailable,
            other => {
                return Err(MalformedIdentity::UnknownStartTimeSource(
                    other.to_owned(),
                ))
            }
        };
        if token.is_empty() {
            return Err(MalformedIdentity::EmptyToken);
        }

        Ok(Self {
            pid,
            start_time,
            token: (*token).to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Identity;
    use super::MalformedIdentity;
    use crate::executor::daemon_identity::RecordedStartTime;

    fn identity(start_time: RecordedStartTime) -> Identity {
        Identity {
            pid: 4242,
            start_time,
            token: "0123456789abcdef0123456789abcdef".to_owned(),
        }
    }

    #[test]
    fn every_start_time_source_round_trips() {
        for start_time in [
            RecordedStartTime::Probe(1_700_145_620),
            RecordedStartTime::Wallclock(1_700_145_620),
            RecordedStartTime::WriterUnavailable,
        ] {
            let original = identity(start_time);
            assert_eq!(
                Identity::parse(&original.render()),
                Ok(original),
                "{start_time:?}"
            );
        }
    }

    /// Four newline-delimited fields, fixed order, one tag byte for the
    /// source — pinned so a change on either side of the language boundary
    /// fails here rather than at run time.
    #[test]
    fn the_wire_format_is_pinned() {
        assert_eq!(
            identity(RecordedStartTime::Probe(1_700_145_620)).render(),
            "4242\n1700145620\np\n0123456789abcdef0123456789abcdef\n"
        );
        assert_eq!(
            identity(RecordedStartTime::Wallclock(17)).render(),
            "4242\n17\nw\n0123456789abcdef0123456789abcdef\n"
        );
        assert_eq!(
            identity(RecordedStartTime::WriterUnavailable).render(),
            "4242\n0\nu\n0123456789abcdef0123456789abcdef\n"
        );
    }

    /// A launcher killed after spawning but before writing leaves the daemon
    /// reading immediate EOF. It must be a failure, not a default.
    #[test]
    fn an_empty_read_is_truncated_rather_than_defaulted() {
        assert_eq!(
            Identity::parse(""),
            Err(MalformedIdentity::Truncated { fields: 0 })
        );
    }

    #[test]
    fn a_short_read_names_how_many_fields_arrived() {
        assert_eq!(
            Identity::parse("4242\n1700145620\n"),
            Err(MalformedIdentity::Truncated { fields: 2 })
        );
    }

    #[test]
    fn each_unparseable_field_is_named() {
        assert_eq!(
            Identity::parse("not-a-pid\n1\np\ntoken\n"),
            Err(MalformedIdentity::UnparseablePid("not-a-pid".to_owned()))
        );
        assert_eq!(
            Identity::parse("1\nnot-a-time\np\ntoken\n"),
            Err(MalformedIdentity::UnparseableStartTime(
                "not-a-time".to_owned()
            ))
        );
        assert_eq!(
            Identity::parse("1\n1\nz\ntoken\n"),
            Err(MalformedIdentity::UnknownStartTimeSource("z".to_owned()))
        );
        assert_eq!(
            Identity::parse("1\n1\np\n\n"),
            Err(MalformedIdentity::EmptyToken)
        );
    }

    /// The writer-unavailable tag ignores its own seconds field rather than
    /// treating a placeholder zero as a real start time.
    #[test]
    fn the_writer_unavailable_tag_does_not_carry_a_start_time() {
        let parsed = Identity::parse("1\n999\nu\ntoken\n");
        assert_eq!(
            parsed.map(|identity| identity.start_time),
            Ok(RecordedStartTime::WriterUnavailable)
        );
    }
}
