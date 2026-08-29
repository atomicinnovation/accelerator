//! The provider-transport isolation tripwire.
//!
//! Provider request construction — the HTTP transport, the certificate and
//! redirect policy, and the environment reads that gate the upload SSRF check —
//! must stay inside the provider client crates. A one-line edit could otherwise
//! move a `reqwest` client, relax `redirect::Policy`, disable certificate
//! verification, or read `ACCELERATOR_TEST_MODE`-style escape hatches from the
//! environment with a green build and no other test noticing.
//!
//! A bare grep for `reqwest` cannot work: `launcher` and `visualiser/server`
//! use it legitimately. The enforcing scan is therefore an allowlist keyed on
//! the workspace-relative crate directory, failing on a new offender rather
//! than on the established ones. Two further scans — for `/rest/api/` and for
//! GraphQL document literals outside their owning crate — are heuristics over
//! source text, so they are reported as warnings rather than failures.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

type TestError = Box<dyn std::error::Error>;

/// Crate directories, workspace-relative, permitted to construct an HTTP
/// transport. `visualiser/server` carries its own non-workspace `reqwest`
/// entry, so a `cli/*/src` glob would never reach it.
const PERMITTED_HTTP_CRATES: &[&str] = &[
    "jira-client",
    "linear-client",
    "launcher",
    "visualiser/server",
    "http-test-support",
];

fn cli_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("the cli workspace root resolves")
}

/// Every `.rs` file under a crate's `src/`, with the walk skipping `target`
/// and any VCS directory — a filesystem walk rather than `git ls-files`, which
/// is blind inside a jj workspace.
fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(root, &mut files);
    files.retain(|path| {
        path.components()
            .any(|component| component.as_os_str() == "src")
    });
    files
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if matches!(name.to_str(), Some("target" | ".git" | ".jj")) {
                continue;
            }
            collect(&path, files);
        } else if path.extension().and_then(std::ffi::OsStr::to_str)
            == Some("rs")
        {
            files.push(path);
        }
    }
}

/// The workspace-relative directory of the nearest ancestor carrying a
/// `Cargo.toml`, normalised with forward slashes so it matches the allowlist.
fn crate_key(root: &Path, file: &Path) -> Option<String> {
    let mut dir = file.parent();
    while let Some(current) = dir {
        if current.join("Cargo.toml").exists() {
            let relative = current.strip_prefix(root).ok()?;
            let key = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            return Some(key);
        }
        if current == root {
            break;
        }
        dir = current.parent();
    }
    None
}

fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.starts_with("//")
}

/// The enforcing scan: returns one message per structural violation.
fn violations(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for file in source_files(root) {
        let Some(key) = crate_key(root, &file) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        let permitted = PERMITTED_HTTP_CRATES.contains(&key.as_str());
        let is_client = key == "jira-client" || key == "linear-client";
        let is_linear_transport_module = key == "linear-client"
            && file.components().any(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some("attach" | "upload")
                ) || matches!(
                    file.file_stem().and_then(std::ffi::OsStr::to_str),
                    Some("attach" | "upload")
                )
            });

        for (number, line) in content.lines().enumerate() {
            if !is_code_line(line) {
                continue;
            }
            let at = |rule: &str| {
                format!("{key}: {} ({}:{})", rule, file.display(), number + 1)
            };
            if !permitted
                && (line.contains("use reqwest") || line.contains("reqwest::"))
            {
                found.push(at("reqwest transport outside a provider crate"));
            }
            if line.contains("danger_accept_invalid_certs")
                || line.contains("danger_accept_invalid_hostnames")
            {
                found.push(at("certificate verification disabled"));
            }
            if is_client
                && (line.contains("redirect::Policy::limited")
                    || line.contains("redirect::Policy::default"))
            {
                found.push(at("a permissive redirect policy in a client"));
            }
            if is_linear_transport_module
                && (line.contains("std::env::var") || line.contains("env::var"))
            {
                found.push(at(
                    "an environment read in the linear upload path — the \
                     loopback exemption must be a constructor parameter",
                ));
            }
        }
    }
    found
}

/// The heuristic scan: request construction that looks provider-specific but
/// lives outside its owning crate. Text-based, so a doc string or error
/// message can trip it — reported as a warning, never a failure.
fn warnings(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for file in source_files(root) {
        let Some(key) = crate_key(root, &file) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (number, line) in content.lines().enumerate() {
            if !is_code_line(line) {
                continue;
            }
            if key != "jira-client"
                && (line.contains("/rest/api/")
                    || line.contains("/rest/agile/"))
            {
                found.push(format!(
                    "{}:{}: a Jira REST path outside jira-client — provider \
                     request construction belongs in the provider crate",
                    file.display(),
                    number + 1
                ));
            }
            let token = line.trim_start().trim_start_matches('"');
            if key != "linear-client"
                && (token.starts_with("query ")
                    || token.starts_with("mutation "))
            {
                found.push(format!(
                    "{}:{}: a GraphQL document outside linear-client — \
                     provider request construction belongs in the provider \
                     crate",
                    file.display(),
                    number + 1
                ));
            }
        }
    }
    found
}

#[test]
fn no_provider_transport_leaks_outside_its_crate() {
    let found = violations(&cli_root());
    for warning in warnings(&cli_root()) {
        eprintln!("warning: {warning}");
    }
    assert!(
        found.is_empty(),
        "provider-transport isolation violated:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_tripwire_fails_on_a_planted_violation() -> Result<(), TestError> {
    let tree = tempfile::Builder::new()
        .prefix("provider-isolation-")
        .tempdir()?;
    let crate_dir = tree.path().join("rogue-crate");
    std::fs::create_dir_all(crate_dir.join("src"))?;
    std::fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n")?;
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        "fn build() {\n    \
         let _ = client.danger_accept_invalid_certs(true);\n}\n",
    )?;

    let found = violations(tree.path());
    assert!(
        found.iter().any(|message| {
            message.contains("certificate verification disabled")
        }),
        "the planted danger_accept_invalid_certs must be caught: {found:?}"
    );
    Ok(())
}

#[test]
fn a_planted_reqwest_import_outside_a_provider_crate_is_caught(
) -> Result<(), TestError> {
    let tree = tempfile::Builder::new()
        .prefix("provider-isolation-reqwest-")
        .tempdir()?;
    let crate_dir = tree.path().join("rogue-crate");
    std::fs::create_dir_all(crate_dir.join("src"))?;
    std::fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n")?;
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        "use reqwest::blocking::Client;\n",
    )?;

    let found = violations(tree.path());
    assert!(
        found
            .iter()
            .any(|message| message.contains("reqwest transport")),
        "the planted reqwest import must be caught: {found:?}"
    );
    Ok(())
}
