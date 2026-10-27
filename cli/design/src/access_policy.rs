//! Whether a source location may be inspected, and what to say when it may
//! not.
//!
//! Three limits are taken positions rather than oversights.
//!
//! The check is **pre-resolution only**: a public hostname resolving to
//! `169.254.169.254` still classifies as public, and nothing re-checks after
//! DNS.
//!
//! The same verdict guards **every `navigate`** — its initial URL and every
//! redirect hop — and every `links` destination per request in the daemon, not
//! only where a crawl begins. It does not cover page subresources, so a hostile
//! page can still reach an internal address through a subresource or a rebased
//! hostname; this is not a full boundary around the navigation surface.
//!
//! It says nothing about **path locations**: a repository path is a valid
//! source location wherever it points, and nothing confirms it lies inside a
//! repository.

use crate::source_location::Scheme;
use crate::source_location::SourceLocation;
use crate::verdict::Verdict;
use network::host_reach;
use network::HostError;
use network::HostReach;

#[must_use]
pub(crate) const fn reach_word(reach: HostReach) -> &'static str {
    match reach {
        HostReach::Loopback => "loopback",
        HostReach::Private => "RFC1918",
        HostReach::LinkLocal => "link-local",
        HostReach::Reserved => "reserved",
        HostReach::Unspecified => "wildcard",
        HostReach::Public => "public",
    }
}

/// The rejection for an authority that could not be canonicalised.
///
/// Only the numeric-encoding and control-character cases name the authority;
/// the others describe a property of the URL rather than of a host.
#[must_use]
pub(crate) fn host_error_message(error: &HostError, authority: &str) -> String {
    match error {
        HostError::Userinfo => "URL contains a userinfo segment (user@host), \
                                which is not permitted."
            .to_owned(),
        HostError::Empty => "URL names no host.".to_owned(),
        HostError::ControlCharacter => format!(
            "host '{authority}' contains a control character, which is not \
             permitted."
        ),
        HostError::NumericEncoding => format!(
            "host '{authority}' uses a numeric IPv4 encoding (decimal, hex, \
             or octal), which is not permitted."
        ),
    }
}

/// The two recovering flags. They only ever travel together and are only
/// meaningful as a pair, so they are one value rather than two bare booleans
/// at every call site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Allowances {
    pub internal: bool,
    pub insecure_scheme: bool,
}

/// Judges `location` under `allowances`.
///
/// A loopback destination is always accepted: it is the local machine talking
/// to itself, and carries neither the internal-network risk `--allow-internal`
/// guards nor the plaintext-interception risk `--allow-insecure-scheme` does.
/// This is the skill's primary documented invocation (`http://localhost:3000`),
/// so it is a rule of its own rather than a consequence of the reach model.
#[must_use]
pub fn evaluate(
    location: &SourceLocation,
    allowances: Allowances,
) -> Verdict<String> {
    let (scheme, host) = match location {
        SourceLocation::Blank | SourceLocation::RepositoryPath(_) => {
            return Verdict::Accepted
        }
        SourceLocation::Url { scheme, host } => (*scheme, host),
    };

    let reach = host_reach::classify(host);
    match reach {
        HostReach::Loopback => return Verdict::Accepted,
        HostReach::Unspecified => {
            return Verdict::Rejected(format!(
                "host '{host}' is a {} address, which names no host. There is \
                 nothing to inventory there.",
                reach_word(reach)
            ))
        }
        HostReach::Private | HostReach::LinkLocal | HostReach::Reserved => {
            if !allowances.internal {
                return Verdict::Rejected(format!(
                    "host '{host}' is a {} address. Pass --allow-internal to \
                     permit.",
                    reach_word(reach)
                ));
            }
            return Verdict::Accepted;
        }
        HostReach::Public => {}
    }

    if scheme == Scheme::Http && !allowances.insecure_scheme {
        return Verdict::Rejected(format!(
            "http:// to public host '{host}' is rejected. Use https:// or \
             pass --allow-insecure-scheme."
        ));
    }
    Verdict::Accepted
}

#[cfg(test)]
mod tests {
    use network::HostReach;

    use super::evaluate;
    use super::reach_word;
    use super::Allowances;
    use crate::source_location;
    use crate::verdict::Verdict;

    type TestError = Box<dyn std::error::Error>;

    fn verdict(
        location: &str,
        allowances: Allowances,
    ) -> Result<Verdict<String>, TestError> {
        Ok(evaluate(&source_location::parse(location)?, allowances))
    }

    fn with_no_flags(location: &str) -> Result<Verdict<String>, TestError> {
        verdict(location, Allowances::default())
    }

    fn internal() -> Allowances {
        Allowances {
            internal: true,
            insecure_scheme: false,
        }
    }

    fn insecure() -> Allowances {
        Allowances {
            internal: false,
            insecure_scheme: true,
        }
    }

    // The scheme and reachability vectors live in the shared corpora
    // (`tests/access_policy_vectors.rs` here, and network's
    // `tests/host_classification_vectors.rs`), which hold the Rust and
    // JavaScript classifiers to the same cases. The tests below pin behaviour
    // the corpus cannot: the loopback carve-out, the unrecoverable wildcard, the
    // exact rejection prose, and the non-URL locations.

    /// The skill's primary documented invocation, carved out ahead of internal
    /// classification.
    #[test]
    fn loopback_is_accepted_on_http_with_no_flags_at_all(
    ) -> Result<(), TestError> {
        for location in [
            "http://localhost:3000",
            "http://127.0.0.1:8080",
            "http://[::1]:3000",
            "http://127.0.0.2",
        ] {
            assert_eq!(
                with_no_flags(location)?,
                Verdict::Accepted,
                "{location}"
            );
        }
        Ok(())
    }

    /// It names no host, so there is nothing for `--allow-internal` to
    /// recover into.
    #[test]
    fn the_unspecified_address_is_rejected_under_every_flag_combination(
    ) -> Result<(), TestError> {
        for allowances in [
            Allowances::default(),
            internal(),
            insecure(),
            Allowances {
                internal: true,
                insecure_scheme: true,
            },
        ] {
            assert!(
                matches!(
                    verdict("http://0.0.0.0", allowances)?,
                    Verdict::Rejected(_)
                ),
                "{allowances:?}"
            );
        }
        Ok(())
    }

    /// The label vocabulary is user-facing text, pinned so a restructuring of
    /// the reach model cannot silently reword a rejection.
    #[test]
    fn the_rfc1918_rejection_names_the_reach_and_the_recovering_flag(
    ) -> Result<(), TestError> {
        let Verdict::Rejected(reason) = with_no_flags("http://10.0.0.1")?
        else {
            return Err("expected a rejection".into());
        };
        assert_eq!(
            reason,
            "host '10.0.0.1' is a RFC1918 address. Pass --allow-internal to \
             permit."
        );
        Ok(())
    }

    #[test]
    fn every_rejected_reach_names_itself_for_the_rejection_message() {
        assert_eq!(reach_word(HostReach::Loopback), "loopback");
        assert_eq!(reach_word(HostReach::Private), "RFC1918");
        assert_eq!(reach_word(HostReach::LinkLocal), "link-local");
        assert_eq!(reach_word(HostReach::Reserved), "reserved");
        assert_eq!(reach_word(HostReach::Unspecified), "wildcard");
    }

    #[test]
    fn a_blank_or_path_location_is_accepted_without_reaching_the_reach_model(
    ) -> Result<(), TestError> {
        assert_eq!(with_no_flags("about:blank")?, Verdict::Accepted);
        assert_eq!(with_no_flags("./src")?, Verdict::Accepted);
        Ok(())
    }
}
