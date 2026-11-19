//! Builds the requested shared fixture families once, before a run's tests.
//!
//! Usage: `shared-fixtures --matrix <tree>`. Each family that fails is printed
//! as `<family>: <cause>` on stderr.
#![allow(clippy::print_stderr)]

use std::env;
use std::process::ExitCode;

use vcs_test_support::family::Family;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::shared_build::FamilyFailure;
use vcs_test_support::shared_build::SharedBuildRequest;
use vcs_test_support::Error;

/// Fault injection for checking the build step: fails the named family's
/// build without running it.
const FORCED_FAILURE_VARIABLE: &str = "ACCELERATOR_SHARED_FIXTURES_FAIL";

fn main() -> ExitCode {
    let request = match SharedBuildRequest::parse(env::args().skip(1)) {
        Ok(request) => request,
        Err(error) => return refuse(&error),
    };
    let failures = match forced_failure() {
        Ok(Some(family)) => vec![forced(family)],
        Ok(None) => request.build(&FixtureLog::from_environment()),
        Err(unrecognised) => return refuse(&unrecognised),
    };
    report(&failures)
}

fn forced_failure() -> Result<Option<Family>, Error> {
    let Some(value) = env::var(FORCED_FAILURE_VARIABLE)
        .ok()
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    Family::ALL
        .into_iter()
        .find(|family| family.name() == value)
        .map(Some)
        .ok_or_else(|| {
            let expected: Vec<&str> =
                Family::ALL.iter().map(|family| family.name()).collect();
            Error::message(format!(
                "{FORCED_FAILURE_VARIABLE}='{value}' is not a family; \
                 expected {}",
                expected.join(" or ")
            ))
        })
}

fn forced(family: Family) -> FamilyFailure {
    FamilyFailure {
        family,
        cause: format!("failed on purpose by {FORCED_FAILURE_VARIABLE}"),
    }
}

fn refuse(error: &Error) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(2)
}

fn report(failures: &[FamilyFailure]) -> ExitCode {
    for FamilyFailure { family, cause } in failures {
        eprintln!("{}: {cause}", family.name());
    }
    if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
