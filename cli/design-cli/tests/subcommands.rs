//! `accelerator-design`'s subcommands, through the binary.
//!
//! Exit codes are asserted explicitly throughout: `scrub-secrets` and
//! `audit-cue-phrases` report a usage error on 2 and a domain rejection on 1,
//! and callers discriminate on the difference.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-design");

/// Runs the binary with an empty environment, so a developer's own
/// `ACCELERATOR_BROWSER_*` cannot leak in.
fn run(arguments: &[&str], environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new(BIN);
    command.args(arguments);
    command.env_clear();
    // Coverage instrumentation writes to the path this names; clearing it
    // makes the child litter its profile into the crate directory instead.
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    for (name, value) in environment {
        command.env(name, value);
    }
    match command.output() {
        Ok(output) => output,
        Err(error) => unreachable!("the binary should run: {error}"),
    }
}

/// A signal death has no code, which no assertion in this suite expects — so
/// it surfaces as an unmatchable sentinel rather than a silent zero.
fn code(arguments: &[&str]) -> i32 {
    run(arguments, &[]).status.code().unwrap_or(-1)
}

fn stderr(arguments: &[&str]) -> String {
    String::from_utf8_lossy(&run(arguments, &[]).stderr).into_owned()
}

fn stdout_of(arguments: &[&str], environment: &[(&str, &str)]) -> String {
    String::from_utf8_lossy(&run(arguments, environment).stdout)
        .trim_end()
        .to_owned()
}

fn validate(location: &str, flags: &[&str]) -> i32 {
    let mut arguments = vec!["validate-source", location];
    arguments.extend_from_slice(flags);
    code(&arguments)
}

fn validate_stderr(location: &str, flags: &[&str]) -> String {
    let mut arguments = vec!["validate-source", location];
    arguments.extend_from_slice(flags);
    stderr(&arguments)
}

#[test]
fn https_and_repository_paths_are_accepted() -> Result<(), TestError> {
    assert_eq!(validate("https://prototype.example.com", &[]), 0);
    let work = tempfile::tempdir()?;
    assert_eq!(validate(&work.path().display().to_string(), &[]), 0);
    Ok(())
}

#[test]
fn every_refused_scheme_exits_one() {
    for location in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,<script>",
        "chrome://settings",
    ] {
        assert_eq!(validate(location, &[]), 1, "{location}");
    }
}

#[test]
fn about_blank_is_accepted_while_other_about_urls_are_not() {
    assert_eq!(validate("about:blank", &[]), 0);
    assert_eq!(validate("about:config", &[]), 1);
}

#[test]
fn a_path_escape_is_rejected_and_a_missing_directory_too() {
    assert_eq!(validate("../../etc/passwd", &[]), 1);
    assert_eq!(validate("./definitely-not-here", &[]), 1);
}

/// A path that exists but is not a directory was evaluated and rejected, so
/// it is exit 1 rather than a usage error.
#[test]
fn a_file_where_a_directory_was_expected_is_a_verdict() -> Result<(), TestError>
{
    let work = tempfile::tempdir()?;
    let file = work.path().join("a.txt");
    fs::write(&file, "x")?;
    assert_eq!(validate(&file.display().to_string(), &[]), 1);
    Ok(())
}

#[test]
fn every_loopback_form_is_accepted_with_no_flags() {
    for location in [
        "http://localhost:8080",
        "http://localhost/",
        "http://127.0.0.1:3000",
        "https://localhost:8443",
        "http://LOCALHOST:8080",
        "http://localhost./",
        "http://localhost:8080/path?q=1",
    ] {
        assert_eq!(validate(location, &[]), 0, "{location}");
    }
}

/// A loopback destination is the local machine talking to itself, so the whole
/// of `127.0.0.0/8` and `::1` is allowed, not two literal strings.
#[test]
fn the_whole_loopback_set_needs_no_allow_internal() {
    for location in ["http://127.0.0.2/", "http://[::1]/", "http://[::1]:8080/"]
    {
        assert_eq!(validate(location, &[]), 0, "{location}");
    }
}

#[test]
fn internal_hosts_need_allow_internal_and_are_recovered_by_it() {
    for location in [
        "http://10.0.0.1/",
        "http://192.168.1.1/",
        "http://169.254.169.254/",
        "http://[fe80::1]/",
        "http://[fe80::1%eth0]/",
        "http://[::ffff:10.0.0.1]/",
    ] {
        assert_eq!(validate(location, &[]), 1, "{location} without the flag");
        assert_eq!(
            validate(location, &["--allow-internal"]),
            0,
            "{location} with --allow-internal"
        );
    }
}

/// Both edges and both just-outside cases are pinned, differentiated by which
/// flag the stderr names, since all four exit 1.
#[test]
fn the_rfc1918_boundary_is_pinned_at_both_edges_and_just_outside() {
    for inside in ["http://172.16.0.1/", "http://172.31.255.255/"] {
        assert_eq!(validate(inside, &[]), 1, "{inside}");
        assert!(
            validate_stderr(inside, &[]).contains("RFC1918"),
            "{inside} must be rejected as RFC1918"
        );
    }
    for outside in ["http://172.15.255.255/", "http://172.32.0.0/"] {
        assert_eq!(validate(outside, &[]), 1, "{outside}");
        let message = validate_stderr(outside, &[]);
        assert!(
            message.contains("--allow-insecure-scheme"),
            "{outside} is a public host, so it must be rejected as an \
             insecure scheme, not as RFC1918"
        );
        assert!(!message.contains("RFC1918"), "{outside}");
    }
}

/// It names no host, so there is nothing for a flag to recover into.
#[test]
fn the_unspecified_address_is_refused_under_every_flag() {
    for location in ["http://0.0.0.0/", "http://[::]/"] {
        for flags in [
            &[][..],
            &["--allow-internal"],
            &["--allow-internal", "--allow-insecure-scheme"],
        ] {
            assert_eq!(validate(location, flags), 1, "{location} {flags:?}");
        }
        assert!(validate_stderr(location, &["--allow-internal"])
            .contains("wildcard"));
    }
}

#[test]
fn no_flag_bypasses_a_numeric_ipv4_encoding() {
    for location in [
        "http://2130706433/",
        "http://0x7f000001/",
        "http://0177.0.0.1/",
        "http://127.0.0.01/",
    ] {
        for flags in [
            &[][..],
            &["--allow-internal"],
            &["--allow-internal", "--allow-insecure-scheme"],
        ] {
            assert_eq!(validate(location, flags), 1, "{location} {flags:?}");
        }
        assert!(
            validate_stderr(location, &[]).contains("numeric IPv4 encoding"),
            "{location} must name the numeric encoding"
        );
    }
}

#[test]
fn a_userinfo_segment_is_rejected_whatever_the_flags() {
    assert_eq!(
        validate("http://user@example.com/", &["--allow-insecure-scheme"]),
        1
    );
    assert_eq!(
        validate(
            "http://user:pass@127.0.0.1@evil.com/",
            &["--allow-internal", "--allow-insecure-scheme"]
        ),
        1
    );
}

#[test]
fn http_to_a_public_host_is_gated_on_the_scheme_flag_alone() {
    assert_eq!(validate("http://example.com/", &[]), 1);
    assert_eq!(validate("http://example.com/", &["--allow-internal"]), 1);
    assert_eq!(
        validate("http://example.com/", &["--allow-insecure-scheme"]),
        0
    );
    assert_eq!(
        validate(
            "http://example.com/",
            &["--allow-internal", "--allow-insecure-scheme"]
        ),
        0
    );

    let message = validate_stderr("http://example.com/", &[]);
    assert!(message.contains("--allow-insecure-scheme"));
    assert!(!message.contains("internal address"));
}

#[test]
fn an_internal_rejection_names_the_flag_and_the_host() {
    let message = validate_stderr("http://10.0.0.1/", &[]);
    assert!(message.contains("--allow-internal"));
    assert!(message.contains("10.0.0.1"));
    assert!(!message.contains("not available in v1"));
}

#[test]
fn an_accepted_location_says_nothing() {
    assert_eq!(validate_stderr("http://localhost:8080", &[]), "");
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    assert_eq!(validate("http://localhost/", &["--alllow-internal"]), 2);
}

#[test]
fn a_missing_location_is_a_usage_error() {
    assert_eq!(code(&["validate-source"]), 2);
}

const HEADER: &str = "ACCELERATOR_BROWSER_AUTH_HEADER";
const USERNAME: &str = "ACCELERATOR_BROWSER_USERNAME";
const PASSWORD: &str = "ACCELERATOR_BROWSER_PASSWORD";
const LOGIN_URL: &str = "ACCELERATOR_BROWSER_LOGIN_URL";

#[test]
fn a_header_takes_precedence_and_warns_about_the_ignored_form_variables() {
    let environment = [
        (HEADER, "Bearer-x"),
        (USERNAME, "u"),
        (PASSWORD, "p"),
        (LOGIN_URL, "https://x/login"),
    ];
    assert_eq!(stdout_of(&["resolve-auth"], &environment), "header");
    let output = run(&["resolve-auth"], &environment);
    assert!(String::from_utf8_lossy(&output.stderr).contains("ignored"));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn all_three_form_variables_resolve_to_form() {
    let environment = [
        (USERNAME, "u"),
        (PASSWORD, "p"),
        (LOGIN_URL, "https://x/login"),
    ];
    assert_eq!(stdout_of(&["resolve-auth"], &environment), "form");
}

#[test]
fn no_variables_resolve_to_none() {
    assert_eq!(stdout_of(&["resolve-auth"], &[]), "none");
}

/// The environment names an intent the tool cannot act on — a malformed
/// invocation rather than a judged input, so exit 2.
#[test]
fn a_partial_form_configuration_is_a_usage_error_naming_what_is_missing() {
    let environment = [(USERNAME, "u"), (PASSWORD, "p")];
    let output = run(&["resolve-auth"], &environment);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains(LOGIN_URL));
}

fn write(
    directory: &Path,
    name: &str,
    body: &str,
) -> Result<String, TestError> {
    let path = directory.join(name);
    fs::write(&path, body)?;
    Ok(path.display().to_string())
}

#[test]
fn a_clean_body_passes_the_scrubber() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let file = write(work.path(), "clean.md", "An ordinary inventory body.\n")?;
    let output = run(&["scrub-secrets", &file], &[(PASSWORD, "hunter2_uniq")]);
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn a_literal_value_is_caught_and_the_variable_named_not_the_value(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let file = write(
        work.path(),
        "leaky.md",
        "The reset link contains hunter2_uniq somewhere.\n",
    )?;
    let output = run(&["scrub-secrets", &file], &[(PASSWORD, "hunter2_uniq")]);
    assert_eq!(output.status.code(), Some(1));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains(PASSWORD));
    assert!(!message.contains("hunter2_uniq"));
    Ok(())
}

/// The value half of a `Name: value` header pair is a needle of its own, since
/// that is the shape a leak most likely takes.
#[test]
fn the_value_half_of_a_header_pair_is_caught() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let file = write(
        work.path(),
        "leaky.md",
        "the request carried Bearer abc123\n",
    )?;
    let output = run(
        &["scrub-secrets", &file],
        &[(HEADER, "Authorization: Bearer abc123")],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains(HEADER));
    Ok(())
}

/// The argument cannot be interpreted as a file to scan, so exit 2 rather than
/// the 1 a scanned-and-rejected body earns.
#[test]
fn scrubbing_a_nonexistent_file_is_a_usage_error() {
    assert_eq!(code(&["scrub-secrets", "/nonexistent-by-construction"]), 2);
}

#[test]
fn scrub_secrets_without_a_file_is_a_usage_error() {
    assert_eq!(code(&["scrub-secrets"]), 2);
}

#[test]
fn every_reason_prints_its_message_and_exits_zero() {
    for reason in design::DowngradeReason::ALL {
        let output = run(&["notify-downgrade", "--reason", reason.key()], &[]);
        assert_eq!(output.status.code(), Some(0), "{reason}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("{}\n", reason.message()),
            "{reason}"
        );
    }
}

#[test]
fn an_unknown_or_missing_reason_is_a_usage_error() {
    assert_eq!(
        code(&["notify-downgrade", "--reason", "not-a-real-reason"]),
        2
    );
    assert_eq!(code(&["notify-downgrade"]), 2);
}

#[test]
fn the_forward_compatible_flags_are_accepted() {
    assert_eq!(
        code(&[
            "notify-downgrade",
            "--from",
            "hybrid",
            "--to",
            "code",
            "--reason",
            "artifact-unavailable",
        ]),
        0
    );
}

const COMPLIANT: &str = "\
# Gap

## Token Drift
We need to migrate the colour scale.

## Component Drift
Users need a five-variant Button.

## Screen Drift
The system must support a redesigned navigation pattern.

## Net-New Features
Implement Search to expose Cmd+K activation and recent-history previews.
";

const NON_COMPLIANT: &str = "\
# Gap

## Token Drift
The colours are different.

## Component Drift
We need a five-variant Button.
";

const LOWERCASE_IMPLEMENT: &str = "\
# Gap

## Token Drift
implement foo to handle the colour migration.
";

const EMPTY_H2: &str = "\
# Gap

## Token Drift

## Component Drift
We need a five-variant Button.
";

fn audit(body: &str) -> Result<Output, TestError> {
    let work = tempfile::tempdir()?;
    let file = write(work.path(), "gaps.md", body)?;
    Ok(run(&["audit-cue-phrases", &file], &[]))
}

#[test]
fn all_four_cue_patterns_pass_the_audit() -> Result<(), TestError> {
    assert_eq!(audit(COMPLIANT)?.status.code(), Some(0));
    Ok(())
}

#[test]
fn an_uncued_section_fails_the_audit_and_is_named() -> Result<(), TestError> {
    let output = audit(NON_COMPLIANT)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Token Drift"));
    Ok(())
}

#[test]
fn a_lowercase_implement_does_not_cue() -> Result<(), TestError> {
    assert_eq!(audit(LOWERCASE_IMPLEMENT)?.status.code(), Some(1));
    Ok(())
}

#[test]
fn an_empty_h2_has_no_prose_to_cue() -> Result<(), TestError> {
    assert_eq!(audit(EMPTY_H2)?.status.code(), Some(0));
    Ok(())
}

/// Exit 2, not 1: there are no offending H2 sections to revise against a file
/// the tool could not read. The skill's retry loop branches on this.
#[test]
fn auditing_a_nonexistent_file_is_a_usage_error() {
    assert_eq!(
        code(&["audit-cue-phrases", "/nonexistent-by-construction"]),
        2
    );
}

#[test]
fn audit_cue_phrases_without_a_file_is_a_usage_error() {
    assert_eq!(code(&["audit-cue-phrases"]), 2);
}

#[test]
fn every_migrated_script_has_a_subcommand() {
    let help =
        String::from_utf8_lossy(&run(&["--help"], &[]).stdout).into_owned();
    for subcommand in [
        "validate-source",
        "resolve-auth",
        "scrub-secrets",
        "notify-downgrade",
        "audit-cue-phrases",
    ] {
        assert!(help.contains(subcommand), "{subcommand} must be listed");
    }
}

#[test]
fn an_unknown_subcommand_is_a_usage_error() {
    assert_eq!(code(&["not-a-subcommand"]), 2);
}
