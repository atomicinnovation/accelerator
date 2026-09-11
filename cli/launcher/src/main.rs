//! The accelerator launcher binary — the composition root: it initialises
//! logging, wires the concrete adapters to the ports, parses the CLI, and
//! dispatches (built-ins in-process, external subcommands via resolve + exec).
//!
//! It is the only module that names `config_adapters`: the `config` port bundle
//! is composed here and handed to `dispatch` behind `config`-crate traits.

use std::ffi::OsString;
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
use accelerator::launch::help::augment_with_subbinaries;
use accelerator::launch::inbound::cli::{CacheAction, Cli, Command};
use accelerator::launch::outbound::exec::UnixExec;
use accelerator::launch::outbound::override_path;
use accelerator::launch::outbound::resolve::cache_root::{
    self, CacheRootConfig,
};
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::manifest::Manifest;
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

/// The signature-verified release manifest, or `None` on any failure so the
/// help path fails open to the built-in listing. Reads only the manifest, no
/// cache root, and bounds the fetch to one short attempt via `Fetcher::for_help`
/// so a slow or unreachable host degrades within the help connect timeout rather
/// than dispatch's retry budget.
///
/// The crypto provider is installed lazily here, so a `version`/`config`/`cache`
/// built-in never pays for TLS it does not use.
fn load_help_manifest() -> Option<Manifest> {
    let _ = install_crypto_provider();
    let keys = TrustedKeys::embedded().ok()?;
    let fetcher = Fetcher::for_help().ok()?;
    let config = ResolverConfig::production(release_base_url(), PathBuf::new());
    let resolver =
        FetchVerifyCacheResolver::with_fetcher(config, keys, fetcher);
    resolver.load_manifest().ok()
}

/// The command listing: the built-ins, plus the manifest's sub-binaries when one
/// loaded. Split from the I/O so the augment-vs-built-ins composition is
/// unit-testable without a signed manifest.
fn build_listing(
    command: clap::Command,
    manifest: Option<&Manifest>,
) -> clap::Command {
    match manifest {
        Some(manifest) => augment_with_subbinaries(command, manifest),
        None => command,
    }
}

/// Render the full command listing to stdout and return `exit`. Every root-help
/// form routes here, so the three entry points converge on one rendering.
fn render_full_listing(exit: ExitCode) -> ExitCode {
    let mut command =
        build_listing(Cli::command(), load_help_manifest().as_ref());
    let _ = command.print_help();
    println!();
    exit
}

/// Whether the collected args are a true top-level help form — top-level
/// `--help`/`-h`, or the bare `help` word — as opposed to a built-in
/// subcommand's own help (`config --help`, `help config`), which clap renders
/// unchanged. The leading-token match keeps a stray trailing token
/// (`--help extra`) on the root form, matching clap's own tolerance; the bare
/// `help` word is root only as the sole token.
fn is_root_help_args(args: &[OsString]) -> bool {
    match args {
        [first, ..] if first == "--help" || first == "-h" => true,
        [only] if only == "help" => true,
        _ => false,
    }
}

/// Why the full listing renders. The two causes differ only in exit code and
/// whether the bare-invocation cue prints, so the cause is modelled rather than
/// carried as loose, independently-settable fields.
#[derive(Debug, PartialEq, Eq)]
enum ListingCause {
    RootHelp,
    MissingSubcommand,
}

/// Where a clap parse outcome routes.
#[derive(Debug, PartialEq, Eq)]
enum HelpRoute {
    FullListing(ListingCause),
    PerCommand,
    UsageError,
}

/// Route a clap parse outcome by its kind and the collected root args. Pure, so
/// the whole truth table is unit-testable.
///
/// The derive marks the required root subcommand `arg_required_else_help`, so a
/// missing root subcommand surfaces as `DisplayHelpOnMissingArgumentOrSubcommand`
/// (not `MissingSubcommand`) with no args. The empty-args slice is what marks the
/// bare root invocation; the same kind with non-empty args is a missing nested
/// subcommand (bare `config`, `config templates`), which keeps its own
/// per-command help. `MissingSubcommand` is matched alongside defensively — a
/// clap change that reverted to it for the bare root would still route here.
fn classify(kind: ErrorKind, args: &[OsString]) -> HelpRoute {
    match kind {
        ErrorKind::DisplayHelp if is_root_help_args(args) => {
            HelpRoute::FullListing(ListingCause::RootHelp)
        }
        ErrorKind::MissingSubcommand
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            if args.is_empty() =>
        {
            HelpRoute::FullListing(ListingCause::MissingSubcommand)
        }
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            HelpRoute::PerCommand
        }
        _ => HelpRoute::UsageError,
    }
}

/// Maps a clap parse outcome to an exit code. clap's own convention exits 2 on
/// a usage error, but this launcher reserves exit 2 for a subcommand refusal,
/// so usage errors are re-mapped to 1 here.
fn handle_parse_error(error: &clap::Error, args: &[OsString]) -> ExitCode {
    match classify(error.kind(), args) {
        HelpRoute::FullListing(cause) => {
            let exit = match cause {
                ListingCause::RootHelp => ExitCode::SUCCESS,
                ListingCause::MissingSubcommand => {
                    // The listing goes to stdout; a bare exit-1 with empty
                    // stderr would read as success, so the cue on stderr keeps
                    // bare invocation diagnosable and distinct from `--help`.
                    eprintln!(
                        "error: a subcommand is required; \
                         run 'accelerator --help' to see available commands"
                    );
                    ExitCode::from(1)
                }
            };
            render_full_listing(exit)
        }
        HelpRoute::PerCommand => {
            // Force stdout for every help/version kind. clap routes
            // `DisplayHelpOnMissingArgumentOrSubcommand` to stderr, which would
            // make a bare `config` print help on a different stream than
            // `config --help`.
            print!("{error}");
            ExitCode::SUCCESS
        }
        HelpRoute::UsageError => {
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
    // External and is delegated to the child. The root args are collected once
    // and threaded into the routing so it reads no process global.
    let root_args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return handle_parse_error(&error, &root_args),
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
    use clap::error::ErrorKind;

    use super::{
        build_listing, classify, handle_dispatch_error, is_root_help_args,
        HelpRoute, ListingCause,
    };

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn only_the_true_root_forms_are_root_help() {
        assert!(is_root_help_args(&args(&["--help"])));
        assert!(is_root_help_args(&args(&["-h"])));
        assert!(is_root_help_args(&args(&["help"])));
        assert!(is_root_help_args(&args(&["--help", "extra"])));
        assert!(!is_root_help_args(&args(&["config", "--help"])));
        assert!(!is_root_help_args(&args(&["help", "config"])));
        assert!(!is_root_help_args(&args(&["version", "--help"])));
        assert!(!is_root_help_args(&args(&[])));
    }

    #[test]
    fn classify_routes_each_kind_and_arg_shape() {
        assert_eq!(
            classify(ErrorKind::DisplayHelp, &args(&["--help"])),
            HelpRoute::FullListing(ListingCause::RootHelp)
        );
        assert_eq!(
            classify(ErrorKind::DisplayHelp, &args(&["-h"])),
            HelpRoute::FullListing(ListingCause::RootHelp)
        );
        assert_eq!(
            classify(ErrorKind::DisplayHelp, &args(&["help"])),
            HelpRoute::FullListing(ListingCause::RootHelp)
        );
        assert_eq!(
            classify(ErrorKind::DisplayHelp, &args(&["--help", "extra"])),
            HelpRoute::FullListing(ListingCause::RootHelp)
        );
        // Bare `accelerator`: the derive's `arg_required_else_help` makes the
        // missing root subcommand this display kind with no args.
        assert_eq!(
            classify(
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                &args(&[])
            ),
            HelpRoute::FullListing(ListingCause::MissingSubcommand)
        );
        // A reverted clap that raised `MissingSubcommand` for the bare root
        // still routes to the listing.
        assert_eq!(
            classify(ErrorKind::MissingSubcommand, &args(&[])),
            HelpRoute::FullListing(ListingCause::MissingSubcommand)
        );
        assert_eq!(
            classify(ErrorKind::DisplayHelp, &args(&["config", "--help"])),
            HelpRoute::PerCommand
        );
        assert_eq!(
            classify(ErrorKind::DisplayVersion, &args(&["version", "--help"])),
            HelpRoute::PerCommand
        );
        // Bare `config` keeps its own per-command help.
        assert_eq!(
            classify(
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                &args(&["config"])
            ),
            HelpRoute::PerCommand
        );
        // A missing nested subcommand (`config templates`) keeps its own help,
        // never the top-level listing.
        assert_eq!(
            classify(
                ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                &args(&["config", "templates"])
            ),
            HelpRoute::PerCommand
        );
        assert_eq!(
            classify(ErrorKind::InvalidValue, &args(&["config", "get"])),
            HelpRoute::UsageError
        );
    }

    #[test]
    fn build_listing_augments_only_when_a_manifest_loaded(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use accelerator::launch::inbound::cli::Cli;
        use accelerator::launch::outbound::resolve::manifest::Manifest;
        use clap::CommandFactory as _;

        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let json = format!(
            "{{\"schema_version\":1,\"version\":\"{VERSION}\",\"binaries\":\
             {{\"zzfixture\":{{\"description\":\"Fixture tool\",\
             \"platforms\":{{}}}}}}}}"
        );
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;

        let mut with = build_listing(Cli::command(), Some(&manifest));
        assert!(with.render_help().to_string().contains("zzfixture"));

        let mut without = build_listing(Cli::command(), None);
        assert!(!without.render_help().to_string().contains("zzfixture"));
        Ok(())
    }

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
