//! The accelerator launcher binary — the composition root: it initialises
//! logging, wires the concrete adapters to the ports, parses the CLI, and
//! dispatches (built-ins in-process, external subcommands via resolve + exec).
//!
//! It is the only module that names `config_adapters`: the `config` port bundle
//! is composed here and handed to `dispatch` behind `config`-crate traits.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser as _};

use accelerator::config_command::core::ConfigStack;
use accelerator::launch::cache;
use accelerator::launch::core::tree::AcquiredTree;
use accelerator::launch::core::{
    acquire_trees, consumes_trees, swallow_under_fail_safe, tree_var,
    ExternalCommand, ResolutionError, ResolveBinary, LAUNCHER_PATH_VAR,
};
use accelerator::launch::dispatch;
use accelerator::launch::help::external_subcommands_section;
use accelerator::launch::inbound::cli::{CacheAction, Cli, Command};
use accelerator::launch::outbound::exec::UnixExec;
use accelerator::launch::outbound::override_path;
use accelerator::launch::outbound::resolve::cache_root::{
    self, CacheRootConfig,
};
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::tree::{
    pins, ExpectedDigests, NoSteps, SystemClock, TreeResolver,
};
use accelerator::launch::outbound::resolve::{
    FetchVerifyCacheResolver, ResolverConfig, HOST_PLATFORM,
};
use accelerator::launch::outbound::tls::install_crypto_provider;
use accelerator::version::core::VersionReporter;
use accelerator::version::outbound::build_metadata::VergenBuildMetadata;
use config::ConfigError;
use config_adapters::LegacyPolicy;

/// The release-download base URL, pinned to the `v{version}` tag and overridable
/// by `ACCELERATOR_RELEASE_BASE_URL`.
fn release_base_url() -> String {
    if let Some(override_url) = std::env::var_os("ACCELERATOR_RELEASE_BASE_URL")
    {
        return override_url.to_string_lossy().into_owned();
    }
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "https://github.com/atomicinnovation/accelerator/releases/download/v{version}"
    )
}

/// The override first, else the real resolver built lazily so built-ins never
/// touch the cache root, TLS, or the network. The rustls crypto provider is
/// installed here rather than in `main`, so a `version` or `config` built-in
/// never pays for capability it does not use.
struct LazyProductionResolver;

impl ResolveBinary for LazyProductionResolver {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        if let Some(path) = override_path(&command.name)? {
            return Ok(path);
        }
        let _ = install_crypto_provider();
        let cache = cache_root::candidate(&CacheRootConfig::from_env(
            config_adapters::plugin_root_from_env(),
        ))?;
        let keys = TrustedKeys::embedded()?;
        let config = ResolverConfig::production(release_base_url(), cache);
        FetchVerifyCacheResolver::new(config, keys)?.resolve(command)
    }
}

/// The external-subcommands help section, or `None` on any failure so `--help`
/// still prints the built-in help. Reads only the manifest, no cache root.
fn help_section() -> Option<String> {
    let _ = install_crypto_provider();
    let keys = TrustedKeys::embedded().ok()?;
    let fetcher = Fetcher::new().ok()?;
    let config = ResolverConfig::production(release_base_url(), PathBuf::new());
    let resolver =
        FetchVerifyCacheResolver::with_fetcher(config, keys, fetcher);
    let manifest = resolver.load_manifest().ok()?;
    external_subcommands_section(&manifest)
}

fn render_augmented_help() -> ExitCode {
    let mut command = Cli::command();
    if let Some(section) = help_section() {
        command = command.after_help(section);
    }
    let _ = command.print_help();
    println!();
    ExitCode::SUCCESS
}

/// Whether a `DisplayHelp` is the top-level help (which the augmentation lists
/// external subcommands into), as opposed to a built-in subcommand's own
/// `--help`, which clap renders unchanged.
fn is_root_help(error: &clap::Error) -> bool {
    if error.kind() != ErrorKind::DisplayHelp {
        return false;
    }
    !matches!(
        std::env::args_os()
            .nth(1)
            .as_deref()
            .and_then(std::ffi::OsStr::to_str),
        Some("version" | "config" | "cache" | "help")
    )
}

/// Maps a clap parse outcome to an exit code. clap's own convention exits 2 on a
/// usage error; the bash config cluster exits 1, and this launcher reserves exit
/// 2 for a subcommand refusal, so usage errors are re-mapped to 1 here. The
/// three non-error display kinds print to stdout and exit 0.
fn handle_parse_error(error: &clap::Error) -> ExitCode {
    match error.kind() {
        ErrorKind::DisplayHelp if is_root_help(error) => {
            render_augmented_help()
        }
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            // Force stdout for every help/version kind. clap routes
            // `DisplayHelpOnMissingArgumentOrSubcommand` to stderr, which would
            // make a bare `config` print help on a different stream than
            // `config --help`.
            print!("{error}");
            ExitCode::SUCCESS
        }
        _ => {
            let _ = error.print();
            ExitCode::from(1)
        }
    }
}

/// The legacy policy the parsed command selects: a read subcommand's
/// `--allow-legacy-layout` flag, else `Reject`.
const fn legacy_policy(command: &Command) -> LegacyPolicy {
    match command {
        Command::Config { action } => action.legacy_policy(),
        Command::Version | Command::Cache { .. } | Command::External(_) => {
            LegacyPolicy::Reject
        }
    }
}

/// Composes the `config` port bundle at `start`'s project root (the current
/// directory when `start` is `None`), applying the resolved legacy policy.
/// Invoked lazily by `dispatch`.
fn compose_stack(
    policy: LegacyPolicy,
    start: Option<PathBuf>,
) -> Result<ConfigStack, ConfigError> {
    let start = match start {
        Some(start) => start,
        None => std::env::current_dir().map_err(|error| ConfigError::Io {
            path: ".".to_owned(),
            detail: error.to_string(),
        })?,
    };
    let composed = config_adapters::compose(&start, policy)?;
    let store = composed
        .store
        .with_plugin_root(config_adapters::plugin_root_from_env());
    Ok(ConfigStack::new(
        Box::new(composed.service),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store),
    ))
}

/// The directory config resolution starts from — the `config paths --doc-types`
/// `[root]` positional, else `None` for the current directory.
fn resolution_start(command: &Command) -> Option<PathBuf> {
    match command {
        Command::Config { action } => {
            action.resolution_root().map(PathBuf::from)
        }
        Command::Version | Command::Cache { .. } | Command::External(_) => None,
    }
}

/// Materialise, verify, repair or prune the tree cache.
///
/// The tree resolver — its `Fetcher`, its trust root, its clock — is built here
/// rather than at `run`'s top, so a `version` or `config` dispatch never
/// constructs it. The dependencies are locals the resolver borrows, so they
/// outlive the `cache::run` call inside this function.
fn run_cache(action: &CacheAction) -> Result<(), kernel::Error> {
    let _ = install_crypto_provider();
    let cache = cache_root::candidate(&CacheRootConfig::from_env(
        config_adapters::plugin_root_from_env(),
    ))?;
    let keys = TrustedKeys::embedded()?;
    let fetcher = Fetcher::new()
        .map_err(|detail| ResolutionError::CacheRootUnavailable { detail })?;
    let clock = SystemClock;
    let steps = NoSteps;
    let resolver = TreeResolver {
        cache_root: cache,
        base_url: release_base_url(),
        platform: HOST_PLATFORM.to_owned(),
        expected_version: env!("CARGO_PKG_VERSION").to_owned(),
        keys: &keys,
        fetcher: &fetcher,
        clock: &clock,
        launcher_id: launcher_id(),
        expected_digests: ExpectedDigests::Compiled,
        waiter_bound: WAITER_BOUND,
        steps: &steps,
    };
    let mut out = std::io::stdout().lock();
    cache::run(action, &resolver, &mut out)
}

/// A per-install identity for the retention claim, derived from the launcher's
/// own content-addressed path so two installs sharing a cache root write
/// distinct claim files. Falls back to the version when the path is unavailable.
fn launcher_id() -> String {
    use std::fmt::Write as _;

    use sha2::{Digest as _, Sha256};
    let seed = std::env::current_exe().map_or_else(
        |_| env!("CARGO_PKG_VERSION").to_owned(),
        |path| path.to_string_lossy().into_owned(),
    );
    let digest = Sha256::digest(seed.as_bytes());
    digest.iter().take(8).fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// The single-flight waiter's deadline: a loser gives up after this rather than
/// hanging for the winner's whole download, emitting a non-sticky
/// materialisation-in-progress the crawl retries on its next invocation.
const WAITER_BOUND: std::time::Duration = std::time::Duration::from_secs(20);

/// Export the tree variables a tree-consuming dispatch's consumer reads, and
/// return the leases pinning each resolved tree against reclamation until the
/// consumer takes over.
///
/// Runs ahead of `dispatch`, so the clearing lands before the resolve path's
/// `ACCELERATOR_<SUB>_BIN` short-circuit could return early: an injected
/// `ACCELERATOR_TREE_<NAME>` is cleared even when the dev-override is in use,
/// after which the consumer reaches `cache ensure` exactly as on a cold cache.
/// Best-effort: a tree that is absent, unpointed or failing its checks simply
/// yields no variable, because "not materialised yet" is the normal state.
fn export_consumed_trees(command: &Command) -> Vec<AcquiredTree> {
    let Command::External(raw) = command else {
        return Vec::new();
    };
    let Some(subcommand) = raw.first().and_then(|arg| arg.to_str()) else {
        return Vec::new();
    };
    if !consumes_trees(subcommand) {
        return Vec::new();
    }

    for artifact in pins::artifact_names() {
        std::env::remove_var(tree_var(artifact));
    }
    std::env::remove_var(LAUNCHER_PATH_VAR);
    if let Ok(exe) = std::env::current_exe() {
        std::env::set_var(LAUNCHER_PATH_VAR, exe);
    }

    let acquired = acquire_consumed_trees().unwrap_or_default();
    for tree in &acquired {
        std::env::set_var(tree_var(&tree.tree.artifact), &tree.tree.path);
    }
    acquired
}

/// Acquire every compiled-in tree that is already materialised, holding a lease
/// on each. `None` on any construction failure — a warm export never errors.
///
/// The `Fetcher` is unused on the `acquire` path (local reads and `lstat`s
/// only, no network, no cache-root write probe) and is dropped before the
/// caller mutates the environment, so the mutation is single-threaded.
fn acquire_consumed_trees() -> Option<Vec<AcquiredTree>> {
    let _ = install_crypto_provider();
    let cache = cache_root::candidate(&CacheRootConfig::from_env(
        config_adapters::plugin_root_from_env(),
    ))
    .ok()?;
    let keys = TrustedKeys::embedded().ok()?;
    let fetcher = Fetcher::new().ok()?;
    let clock = SystemClock;
    let steps = NoSteps;
    let resolver = TreeResolver {
        cache_root: cache,
        base_url: release_base_url(),
        platform: HOST_PLATFORM.to_owned(),
        expected_version: env!("CARGO_PKG_VERSION").to_owned(),
        keys: &keys,
        fetcher: &fetcher,
        clock: &clock,
        launcher_id: launcher_id(),
        expected_digests: ExpectedDigests::Compiled,
        waiter_bound: WAITER_BOUND,
        steps: &steps,
    };
    let names: Vec<&str> = pins::artifact_names();
    acquire_trees(&resolver, &names).ok()
}

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    // Held until `dispatch` execs the consumer: on success the process image is
    // replaced and no destructor runs, so the leases pin their trees against
    // reclamation right up to the handover.
    let _tree_leases = export_consumed_trees(&cli.command);
    let reporter = VersionReporter::new(VergenBuildMetadata);
    let resolver = LazyProductionResolver;
    let executor = UnixExec;
    let policy = legacy_policy(&cli.command);
    let start = resolution_start(&cli.command);
    dispatch(
        cli,
        &reporter,
        &resolver,
        &executor,
        move || compose_stack(policy, start),
        run_cache,
    )
}

fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

/// The exit code for a failed `run()`: an availability-class failure from
/// resolving/exec'ing an external subcommand that forwarded `--fail-safe`
/// exits 0 silently (bar a `tracing::warn!` diagnostic); every other failure
/// reports and exits through [`report`] as before.
fn handle_dispatch_error(error: &kernel::Error, command: &Command) -> ExitCode {
    if let Command::External(args) = command {
        if swallow_under_fail_safe(error, args) {
            tracing::warn!(
                %error,
                "external dispatch failed under --fail-safe; exiting 0"
            );
            return ExitCode::SUCCESS;
        }
    }
    report(error)
}

fn main() -> ExitCode {
    // try_parse so the top-level `--help` can be intercepted and augmented, and
    // a usage error re-mapped from clap's exit 2 to 1; a `foo --help` routes to
    // External and is delegated to the child.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return handle_parse_error(&error),
    };

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => handle_dispatch_error(&error, &cli.command),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::process::ExitCode;

    use accelerator::launch::core::{
        run_external, ExecBinary, ExternalCommand, ResolutionError,
        ResolveBinary,
    };
    use accelerator::launch::inbound::cli::Command;

    use super::handle_dispatch_error;

    struct FailingResolver<F>(F);

    impl<F: Fn() -> ResolutionError> ResolveBinary for FailingResolver<F> {
        fn resolve(
            &self,
            _command: &ExternalCommand,
        ) -> Result<PathBuf, ResolutionError> {
            Err((self.0)())
        }
    }

    struct UnreachableExec;

    impl ExecBinary for UnreachableExec {
        fn exec(&self, _program: &Path, _args: &[OsString]) -> ResolutionError {
            unreachable!("a failed resolve must never reach exec")
        }
    }

    fn dispatch_error(
        make_error: impl Fn() -> ResolutionError,
    ) -> kernel::Error {
        let command = ExternalCommand {
            name: OsString::from("vcs"),
            args: vec![],
        };
        run_external(&FailingResolver(make_error), &UnreachableExec, &command)
            .into()
    }

    fn availability_failure() -> ResolutionError {
        ResolutionError::Fetch {
            target: "vcs".to_owned(),
            url: "https://example.test/vcs".to_owned(),
        }
    }

    fn integrity_failure() -> ResolutionError {
        ResolutionError::ChecksumMismatch {
            asset: "vcs".to_owned(),
            expected: "a".repeat(64),
            actual: "b".repeat(64),
        }
    }

    #[test]
    fn an_availability_failure_exits_zero_when_fail_safe_is_forwarded() {
        let error = dispatch_error(availability_failure);
        let command = Command::External(vec![OsString::from("--fail-safe")]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::SUCCESS);
    }

    #[test]
    fn an_availability_failure_exits_failure_without_fail_safe() {
        let error = dispatch_error(availability_failure);
        let command = Command::External(vec![]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::FAILURE);
    }

    #[test]
    fn an_integrity_failure_exits_two_even_when_fail_safe_is_forwarded() {
        let error = dispatch_error(integrity_failure);
        let command = Command::External(vec![OsString::from("--fail-safe")]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::from(2));
    }

    #[test]
    fn an_integrity_failure_exits_two_without_fail_safe() {
        let error = dispatch_error(integrity_failure);
        let command = Command::External(vec![]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::from(2));
    }
}
