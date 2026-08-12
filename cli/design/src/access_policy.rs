//! Whether a source location may be inspected, and what to say when it may
//! not.

use crate::host_reach;
use crate::host_reach::HostReach;
use crate::source_location::Scheme;
use crate::source_location::SourceLocation;
use crate::verdict::Verdict;

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
                reach.description()
            ))
        }
        HostReach::Private | HostReach::LinkLocal | HostReach::Reserved => {
            if !allowances.internal {
                return Verdict::Rejected(format!(
                    "host '{host}' is a {} address. Pass --allow-internal to \
                     permit.",
                    reach.description()
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
    use super::evaluate;
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

    #[test]
    fn https_to_a_public_host_needs_no_flag() -> Result<(), TestError> {
        assert_eq!(with_no_flags("https://example.com")?, Verdict::Accepted);
        Ok(())
    }

    #[test]
    fn http_to_a_public_host_needs_the_insecure_scheme_flag(
    ) -> Result<(), TestError> {
        let rejection = with_no_flags("http://example.com")?;
        assert!(matches!(rejection, Verdict::Rejected(_)));
        assert_eq!(
            verdict("http://example.com", insecure())?,
            Verdict::Accepted
        );
        Ok(())
    }

    /// The skill's primary documented invocation, and the carve-out the shell
    /// applies before internal classification.
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

    #[test]
    fn every_internal_reach_is_recovered_by_allow_internal(
    ) -> Result<(), TestError> {
        for location in [
            "http://10.0.0.1",
            "http://169.254.169.254",
            "http://100.64.0.1",
            "http://[fd00::1]",
            "http://[::ffff:10.0.0.1]",
        ] {
            assert!(
                matches!(with_no_flags(location)?, Verdict::Rejected(_)),
                "{location} must be rejected without --allow-internal"
            );
            assert_eq!(
                verdict(location, internal())?,
                Verdict::Accepted,
                "{location} must be recovered by --allow-internal"
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

    /// The label vocabulary is user-facing text, and the shell's own wording
    /// for the RFC 1918 case must survive the restructuring.
    #[test]
    fn the_rfc1918_rejection_keeps_the_shell_s_wording() -> Result<(), TestError>
    {
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
    fn a_blank_or_path_location_is_accepted_without_reaching_the_reach_model(
    ) -> Result<(), TestError> {
        assert_eq!(with_no_flags("about:blank")?, Verdict::Accepted);
        assert_eq!(with_no_flags("./src")?, Verdict::Accepted);
        Ok(())
    }
}
