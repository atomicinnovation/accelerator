//! `install_crypto_provider()` is called only inside the production resolver's
//! `resolve()` (the external-subcommand path) and `help_section()` (top-level
//! `--help`). A built-in — `version` or any `config` subcommand — is dispatched
//! in-process and must never call `resolve()`, so it never installs the rustls
//! crypto provider and never pays for capability it does not use.
//!
//! A black-box subprocess cannot observe the process-global provider, so this
//! drives the library `dispatch` directly with a spy `ResolveBinary` and asserts
//! the spy is never consulted for a built-in — the mock-`ResolveBinary` dispatch
//! harness the plan calls for.
#![allow(clippy::panic)]

use std::cell::Cell;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser as _;

use accelerator::config_command::core::ConfigStack;
use accelerator::launch::core::{
    ExecBinary, ExternalCommand, ResolutionError, ResolveBinary,
};
use accelerator::launch::dispatch;
use accelerator::launch::inbound::cli::Cli;
use accelerator::version::core::{ReportVersion, VersionReport};
use config_adapters::LegacyPolicy;

type TestResult = Result<(), Box<dyn Error>>;

/// Records whether `resolve` was ever called. A built-in dispatch must leave it
/// `false`.
struct SpyResolver {
    called: Cell<bool>,
}

impl ResolveBinary for SpyResolver {
    fn resolve(
        &self,
        _command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        self.called.set(true);
        Err(ResolutionError::EmptyCommand)
    }
}

/// Must never run: a built-in exec's nothing.
struct PanicExec;

impl ExecBinary for PanicExec {
    fn exec(&self, _program: &Path, _args: &[OsString]) -> ResolutionError {
        panic!("a built-in must not exec an external binary");
    }
}

struct StubReporter;

impl ReportVersion for StubReporter {
    fn report(&self) -> VersionReport {
        VersionReport {
            version: "0".to_owned(),
            commit_sha: "0".to_owned(),
            build_date: "0".to_owned(),
            target_triple: "0".to_owned(),
        }
    }
}

/// A throwaway workspace rooted at a `.git` marker, carrying a minimal team
/// config so `config path` resolves without escaping into the real tree.
fn fixture() -> Result<PathBuf, Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("crypto-provider-{}", std::process::id()));
    fs::create_dir_all(root.join(".git"))?;
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(
        root.join(".accelerator/config.md"),
        "---\npaths:\n  work: custom/work\n---\n",
    )?;
    Ok(root)
}

#[test]
fn version_never_consults_the_resolver() -> TestResult {
    let spy = SpyResolver {
        called: Cell::new(false),
    };
    let cli = Cli::try_parse_from(["accelerator", "version"])?;
    dispatch(&cli, &StubReporter, &spy, &PanicExec, || {
        panic!("version must not compose the config stack")
    })?;
    assert!(!spy.called.get(), "version reached the resolver");
    Ok(())
}

#[test]
fn config_path_never_consults_the_resolver() -> TestResult {
    let root = fixture()?;
    let spy = SpyResolver {
        called: Cell::new(false),
    };
    let cli = Cli::try_parse_from(["accelerator", "config", "path", "work"])?;
    dispatch(&cli, &StubReporter, &spy, &PanicExec, || {
        let composed = config_adapters::compose(&root, LegacyPolicy::Reject)?;
        let store = composed.store;
        Ok(ConfigStack::new(
            Box::new(composed.service),
            Box::new(store.clone()),
            Box::new(store.clone()),
            Box::new(store.clone()),
            Box::new(store.clone()),
            Box::new(store.clone()),
            Box::new(store),
        ))
    })?;
    assert!(!spy.called.get(), "config path reached the resolver");
    Ok(())
}
