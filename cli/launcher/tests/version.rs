//! Black-box tests of the compiled `accelerator` binary.
//!
//! The version fields are asserted against the values baked into THIS package at
//! build time — `env!("CARGO_PKG_VERSION")` and the same `option_env!("VERGEN_*")`
//! expressions the adapter uses (cargo:rustc-env vars apply to the package's
//! integration tests too). So the equalities are deterministic and about the
//! built artefact, not live git state or the wall clock.

use std::collections::HashSet;
use std::error::Error;
use std::process::{Command, Output};

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const UNKNOWN: &str = "unknown";

fn run(args: &[&str], env: &[(&str, &str)]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator"));
    command.env_remove("ACCELERATOR_LOG");
    // Make external-dispatch tests hermetic w.r.t. the host: no inherited plugin
    // root, cache override, or release base URL leaks into resolution.
    command.env_remove("CLAUDE_PLUGIN_ROOT");
    command.env_remove("ACCELERATOR_CACHE_DIR");
    command.env_remove("ACCELERATOR_RELEASE_BASE_URL");
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    Ok(command.output()?)
}

fn field(
    lines: &[String],
    index: usize,
    prefix: &str,
) -> Result<String, Box<dyn Error>> {
    let line = lines
        .get(index)
        .ok_or_else(|| format!("missing output line {index}"))?;
    let value = line
        .strip_prefix(prefix)
        .ok_or_else(|| format!("line {line:?} missing prefix {prefix:?}"))?;
    Ok(value.to_owned())
}

type Fields = (String, String, String, String);

fn version_fields(stdout: &str) -> Result<Fields, Box<dyn Error>> {
    let lines: Vec<String> = stdout.lines().map(str::to_owned).collect();
    let version = field(&lines, 0, "accelerator ")?;
    let commit_sha = field(&lines, 1, "commit: ")?;
    let build_date = field(&lines, 2, "built:  ")?;
    let target_triple = field(&lines, 3, "target: ")?;
    Ok((version, commit_sha, build_date, target_triple))
}

#[test]
fn version_reports_build_baked_metadata() -> Result<(), Box<dyn Error>> {
    let output = run(&["version"], &[])?;
    assert!(
        output.status.success(),
        "`accelerator version` exited non-zero: {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout)?;
    let (version, commit_sha, build_date, target_triple) =
        version_fields(&stdout)?;

    // Each field equals the build-baked value, so plumbing to stdout is proven
    // by construction in every build context, git present or not.
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert_eq!(commit_sha, option_env!("VERGEN_GIT_SHA").unwrap_or(UNKNOWN));
    assert_eq!(
        build_date,
        option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or(UNKNOWN)
    );
    assert_eq!(
        target_triple,
        option_env!("VERGEN_CARGO_TARGET_TRIPLE").unwrap_or(UNKNOWN)
    );

    // Symmetry-breaking guards so the equalities are not tautologies.
    for value in [&version, &commit_sha, &build_date, &target_triple] {
        assert!(!value.is_empty(), "a printed field was empty");
    }
    let distinct: HashSet<&str> = [
        version.as_str(),
        commit_sha.as_str(),
        build_date.as_str(),
        target_triple.as_str(),
    ]
    .into_iter()
    .collect();
    assert_eq!(distinct.len(), 4, "two printed fields are identical");

    // Only commit_sha is git-derived; CI and local dev always build inside a git
    // working tree, so it is a real SHA, not the degraded sentinel.
    assert_ne!(commit_sha, UNKNOWN);
    // build_date and target_triple are non-git vergen outputs Cargo always
    // supplies — a sentinel here is a real emit bug, so the guard is unconditional.
    assert_ne!(build_date, UNKNOWN);
    assert_ne!(target_triple, UNKNOWN);

    // The build date is a real RFC 3339 timestamp, not in the future. No lower
    // bound: an incremental build legitimately carries an earlier real timestamp.
    let parsed = OffsetDateTime::parse(&build_date, &Rfc3339)?;
    assert!(
        parsed <= OffsetDateTime::now_utc(),
        "build date is in the future: {build_date}"
    );

    Ok(())
}

#[test]
fn version_emits_the_log_line_at_debug() -> Result<(), Box<dyn Error>> {
    let output = run(&["version"], &[("ACCELERATOR_LOG", "debug")])?;
    assert!(
        output.status.success(),
        "exited non-zero: {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout)?;
    version_fields(&stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("reporting version"),
        "stderr missing the log line: {stderr:?}"
    );
    Ok(())
}

#[test]
fn version_is_quiet_at_the_default_level() -> Result<(), Box<dyn Error>> {
    let output = run(&["version"], &[])?;
    assert!(
        output.status.success(),
        "exited non-zero: {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout)?;
    version_fields(&stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        !stderr.contains("reporting version"),
        "debug line leaked at the default level: {stderr:?}"
    );
    Ok(())
}

#[test]
fn a_malformed_filter_exits_non_zero() -> Result<(), Box<dyn Error>> {
    let output = run(&["version"], &[("ACCELERATOR_LOG", "bad=notalevel")])?;
    assert!(
        !output.status.success(),
        "expected a non-zero exit for a malformed filter"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("invalid log filter"),
        "stderr missing the error: {stderr:?}"
    );
    Ok(())
}

#[test]
fn an_unresolvable_subcommand_exits_non_zero_with_a_named_step(
) -> Result<(), Box<dyn Error>> {
    // With `external_subcommand`, an unknown subcommand no longer trips clap's
    // hard rejection — it routes to resolution. With no override and no plugin
    // root, resolution fails closed at the cache-root step (before any network)
    // and the launcher exits non-zero with that named diagnostic — never a panic
    // or silent success. The subcommand-naming failure modes (asset-not-found,
    // release-unavailable) are covered hermetically in `resolution.rs`.
    let output = run(&["definitely-not-a-command"], &[])?;
    assert!(
        !output.status.success(),
        "expected a non-zero exit for an unresolvable subcommand"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("CLAUDE_PLUGIN_ROOT"),
        "stderr missing the named resolution step: {stderr:?}"
    );
    Ok(())
}
