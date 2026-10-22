//! The accelerator launcher binary — the composition root: it initialises
//! logging, wires the concrete adapters to the ports, parses the CLI, and
//! dispatches (built-ins in-process, external subcommands via resolve + exec).
//!
//! It is the only module that names `config_adapters`: the `config` port bundle
//! is composed here and handed to `dispatch` behind `config`-crate traits.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::error::ErrorKind;
use clap::{CommandFactory as _, FromArgMatches as _};

use accelerator::config_command::core::{ConfigStack, ConsentPorts};
use accelerator::launch::cache;
use accelerator::launch::core::tree::AcquiredTree;
use accelerator::launch::core::{
    acquire_trees, consumes_trees, tree_var, DispatchFailureExit,
    DispatchFailurePolicy, ExternalCommand, ReResolution, ResolutionError,
    ResolveBinary, LAUNCHER_PATH_VAR,
};
use accelerator::launch::dispatch;
use accelerator::launch::help::{self, augment_with_subbinaries};
use accelerator::launch::inbound::cli::{
    CacheAction, Cli, Command, ConfigAction, SummaryFormat,
};
use accelerator::launch::outbound::exec::UnixExec;
use accelerator::launch::outbound::override_path;
use accelerator::launch::outbound::resolve::cache_root::CacheRootConfig;
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::filesystem::SystemFilesystem;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::manifest::Manifest;
use accelerator::launch::outbound::resolve::ownership::safe_root::TrustedOwners;
use accelerator::launch::outbound::resolve::ownership::ProcessOwnership;
use accelerator::launch::outbound::resolve::prune_report::NoPruneSteps;
use accelerator::launch::outbound::resolve::retention::ports::CachePorts;
use accelerator::launch::outbound::resolve::retention::residue::KillZeroProbe;
use accelerator::launch::outbound::resolve::tree::lease::{
    FlockProbe, HeldLeases,
};
use accelerator::launch::outbound::resolve::tree::locks::FlockLocks;
use accelerator::launch::outbound::resolve::tree::reap::UnobservedRemovals;
use accelerator::launch::outbound::resolve::tree::{
    pins, ExpectedDigests, NoSteps, SystemClock,
};
use accelerator::launch::outbound::resolve::{
    FetchVerifyCacheResolver, NothingAfterStore, ResolverConfig, ResolverPorts,
};
use accelerator::launch::outbound::tls::install_crypto_provider;
use accelerator::launch::start_up::{
    start_up, DispatchConfig, StartUp, TreeConfig,
};
use accelerator::version::core::VersionReporter;
use accelerator::version::outbound::build_metadata::VergenBuildMetadata;
use config::ConfigError;
use config_adapters::credentials::{
    SystemEnvironment, PERSONAL_CONFIG_RELATIVE,
};
use config_adapters::{FileConfigStore, LegacyPolicy, TrackedConfigFile};
use vcs_adapters::InProcessTracking;

// A release build that turns debug assertions back on slips past this guard, so
// `main` reads the marker below: stripping and LTO then keep its bytes in the
// binary, where a scan can find them.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
const TEST_LOOPBACK_MARKER: &str = "ACCELERATOR_LAUNCHER_TEST_LOOPBACK_MARKER";

/// Which key a release is trusted under.
#[derive(Debug, PartialEq, Eq)]
enum ReleaseTrust {
    Embedded,
    #[cfg(feature = "test-loopback")]
    TestKey {
        path: PathBuf,
        contents: String,
    },
}

/// Only a plain-`http` release on `127.0.0.1` may be a test release, so no
/// host a user could name, and no lookalike of loopback, is ever admitted.
#[cfg(feature = "test-loopback")]
fn admits_test_release(base_url: &str) -> bool {
    reqwest::Url::parse(base_url).is_ok_and(|url| {
        url.scheme() == "http" && url.host_str() == Some("127.0.0.1")
    })
}

#[cfg(feature = "test-loopback")]
fn release_trust_for(base_url: &str, key: Option<&Path>) -> ReleaseTrust {
    let Some(key) = key else {
        return ReleaseTrust::Embedded;
    };
    if admits_test_release(base_url) {
        if let Ok(contents) = std::fs::read_to_string(key) {
            return ReleaseTrust::TestKey {
                path: key.to_path_buf(),
                contents,
            };
        }
    }
    tracing::warn!(
        key = %key.display(),
        "ignoring ACCELERATOR_TEST_RELEASE_PUBLIC_KEY"
    );
    ReleaseTrust::Embedded
}

#[cfg(feature = "test-loopback")]
fn release_trust() -> ReleaseTrust {
    let key = std::env::var_os("ACCELERATOR_TEST_RELEASE_PUBLIC_KEY")
        .map(PathBuf::from);
    release_trust_for(&release_base_url(), key.as_deref())
}

#[cfg(not(feature = "test-loopback"))]
const fn release_trust() -> ReleaseTrust {
    ReleaseTrust::Embedded
}

fn release_keys() -> Result<TrustedKeys, ResolutionError> {
    match release_trust() {
        ReleaseTrust::Embedded => TrustedKeys::embedded(),
        #[cfg(feature = "test-loopback")]
        ReleaseTrust::TestKey { path, contents } => {
            tracing::warn!(
                "{TEST_LOOPBACK_MARKER}: trusting the test release key at {}",
                path.display()
            );
            TrustedKeys::from_public_key_files(&[contents.as_str()])
        }
    }
}

#[cfg(feature = "test-loopback")]
fn release_fetcher_for(base_url: &str) -> Result<Fetcher, String> {
    if admits_test_release(base_url) {
        Fetcher::admitting_loopback_http()
    } else {
        Fetcher::new()
    }
}

#[cfg(feature = "test-loopback")]
fn release_fetcher() -> Result<Fetcher, String> {
    release_fetcher_for(&release_base_url())
}

#[cfg(not(feature = "test-loopback"))]
fn release_fetcher() -> Result<Fetcher, String> {
    Fetcher::new()
}

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

/// The override first, else the real resolver built lazily over the root
/// `start_up` identified, so built-ins never touch TLS or the network. The
/// rustls crypto provider is installed here rather than in `main`, so a
/// `version` or `config` built-in never pays for capability it does not use.
struct LazyProductionResolver<'a> {
    start: &'a StartUp,
    consumed: &'a [AcquiredTree],
}

impl ResolveBinary for LazyProductionResolver<'_> {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        if let Some(path) = override_path(&command.name)? {
            return Ok(path);
        }
        production_resolver(self.start, self.consumed)?.resolve(command)
    }

    fn re_resolve(
        &self,
        command: &ExternalCommand,
        missing: &Path,
    ) -> ReResolution {
        match override_path(&command.name) {
            Ok(Some(_)) => ReResolution::Declined,
            Ok(None) => match production_resolver(self.start, self.consumed) {
                Ok(resolver) => resolver.re_resolve(command, missing),
                Err(error) => ReResolution::Failed(error),
            },
            Err(error) => ReResolution::Failed(error),
        }
    }
}

fn production_resolver(
    start: &StartUp,
    consumed: &[AcquiredTree],
) -> Result<FetchVerifyCacheResolver, ResolutionError> {
    let _ = install_crypto_provider();
    let keys = release_keys()?;
    let fetcher = release_fetcher()
        .map_err(|detail| ResolutionError::CacheRootUnavailable { detail })?;
    start.dispatch_resolver(
        DispatchConfig {
            base_url: release_base_url(),
            keys,
            fetcher,
        },
        consumed,
    )
}

fn production_ports() -> Arc<CachePorts> {
    Arc::new(CachePorts {
        filesystem: Arc::new(SystemFilesystem),
        ownership: Arc::new(ProcessOwnership::default()),
        clock: Arc::new(SystemClock),
        locks: Arc::new(FlockLocks),
        steps: Arc::new(NoPruneSteps),
        bootstrap_holders: Arc::new(KillZeroProbe),
        lease_liveness: Arc::new(FlockProbe),
        pins: Arc::new(ExpectedDigests::Compiled),
        removals: Arc::new(UnobservedRemovals),
    })
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
    let resolver = FetchVerifyCacheResolver::with_fetcher(
        config,
        keys,
        fetcher,
        ResolverPorts {
            filesystem: Arc::new(SystemFilesystem),
            after_store: Arc::new(NothingAfterStore),
            held_leases: HeldLeases::none(),
        },
    );
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

/// Who warns about an ignored personal file. The hook summary carries the
/// warning in its envelope, so a direct warning would repeat it.
#[derive(Clone, Copy)]
enum PersonalFileWarning {
    Direct,
    InTheHookEnvelope,
}

const fn personal_file_warning(command: &Command) -> PersonalFileWarning {
    match command {
        Command::Config {
            action:
                ConfigAction::Summary {
                    format: Some(SummaryFormat::Hook),
                    ..
                },
        } => PersonalFileWarning::InTheHookEnvelope,
        _ => PersonalFileWarning::Direct,
    }
}

/// Composes the `config` port bundle at `start`'s project root (the current
/// directory when `start` is `None`), applying the resolved legacy policy.
/// Invoked lazily by `dispatch`.
fn compose_stack(
    policy: LegacyPolicy,
    start: Option<PathBuf>,
    warning: PersonalFileWarning,
) -> Result<ConfigStack, ConfigError> {
    let start = match start {
        Some(start) => start,
        None => std::env::current_dir().map_err(|error| ConfigError::Io {
            path: ".".to_owned(),
            detail: error.to_string(),
        })?,
    };
    let composed = config_adapters::compose(&start, policy)?;
    if matches!(warning, PersonalFileWarning::Direct) {
        composed.report_ignored_personal_file();
    }
    let consent = ConsentPorts {
        tracking: Box::new(TrackedConfigFile(InProcessTracking)),
        environment: Box::new(SystemEnvironment),
        personal_config: FileConfigStore::discover_root(&start)
            .join(PERSONAL_CONFIG_RELATIVE),
    };
    let plugin_root = config_adapters::plugin_root_from_env();
    let screened = composed.screened.with_plugin_root(plugin_root.clone());
    let store = composed.store.with_plugin_root(plugin_root);
    Ok(ConfigStack::new(
        Box::new(composed.service),
        Box::new(screened.clone()),
        Box::new(screened),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store.clone()),
        Box::new(store),
        consent,
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
fn run_cache(
    action: &CacheAction,
    start: &StartUp,
) -> Result<(), kernel::Error> {
    let _ = install_crypto_provider();
    let keys = release_keys()?;
    let fetcher = release_fetcher()
        .map_err(|detail| ResolutionError::CacheRootUnavailable { detail })?;
    let resolver =
        start.tree_resolver(production_tree_config(&keys, &fetcher))?;
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    cache::run(action, &resolver, start.retention(), &mut out, &mut err)
}

fn production_tree_config<'a>(
    keys: &'a TrustedKeys,
    fetcher: &'a Fetcher,
) -> TreeConfig<'a> {
    TreeConfig {
        base_url: release_base_url(),
        keys,
        fetcher,
        expected_digests: ExpectedDigests::Compiled,
        waiter_bound: WAITER_BOUND,
        steps: &NoSteps,
    }
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
fn export_consumed_trees(
    command: &Command,
    start: &StartUp,
) -> Vec<AcquiredTree> {
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

    let acquired = acquire_consumed_trees(start).unwrap_or_default();
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
fn acquire_consumed_trees(start: &StartUp) -> Option<Vec<AcquiredTree>> {
    let _ = install_crypto_provider();
    let keys = release_keys().ok()?;
    let fetcher = release_fetcher().ok()?;
    let resolver = start
        .tree_resolver(production_tree_config(&keys, &fetcher))
        .ok()?;
    let names: Vec<&str> = pins::artifact_names();
    acquire_trees(&resolver, &names).ok()
}

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let start = start_up(
        &CacheRootConfig::from_env(config_adapters::plugin_root_from_env()),
        TrustedOwners::from_env(),
        production_ports(),
    );
    // Held until `dispatch` execs the consumer: on success the process image is
    // replaced and no destructor runs, so the leases pin their trees against
    // reclamation right up to the handover.
    let consumed = export_consumed_trees(&cli.command, &start);
    let reporter = VersionReporter::new(VergenBuildMetadata);
    let resolver = LazyProductionResolver {
        start: &start,
        consumed: &consumed,
    };
    let executor = UnixExec;
    let policy = legacy_policy(&cli.command);
    let resolution_root = resolution_start(&cli.command);
    let warning = personal_file_warning(&cli.command);
    dispatch(
        cli,
        &reporter,
        &resolver,
        &executor,
        move || compose_stack(policy, resolution_root, warning),
        |action| run_cache(action, &start),
    )
}

fn report(error: &kernel::Error) -> ExitCode {
    print_diagnostic(error);
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn print_diagnostic(error: &kernel::Error) {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
}

/// The exit code for a failed `run()`: an external subcommand's failure exits
/// as its forwarded [`DispatchFailurePolicy`] decides; every other failure
/// reports and exits through [`report`].
fn handle_dispatch_error(error: &kernel::Error, command: &Command) -> ExitCode {
    let Command::External(args) = command else {
        return report(error);
    };
    match DispatchFailurePolicy::forwarded_in(args).exit_for(error) {
        DispatchFailureExit::Swallowed => {
            tracing::warn!(
                %error,
                "external dispatch failed under --fail-safe; exiting 0"
            );
            ExitCode::SUCCESS
        }
        DispatchFailureExit::Reported(code) => {
            print_diagnostic(error);
            ExitCode::from(code)
        }
    }
}

fn parse_cli() -> Result<Cli, clap::Error> {
    let matches = Cli::command()
        .mut_subcommand("config", |config| {
            config.after_help(help::recognised_keys())
        })
        .try_get_matches_from(std::env::args_os())?;
    Cli::from_arg_matches(&matches)
}

fn main() -> ExitCode {
    #[cfg(feature = "test-loopback")]
    std::hint::black_box(TEST_LOOPBACK_MARKER);
    // A fallible parse so the top-level `--help` can be intercepted and
    // augmented, and a usage error re-mapped from clap's exit 2 to 1; a
    // `foo --help` routes to External and is delegated to the child. The root
    // args are collected once and threaded into the routing so it reads no
    // process global.
    let root_args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let cli = match parse_cli() {
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
        run_external, ExecBinary, ExternalCommand, ReResolution,
        ResolutionError, ResolveBinary,
    };
    use accelerator::launch::inbound::cli::Command;
    use accelerator::launch::outbound::resolve::cache_root::CacheRootConfig;
    use accelerator::launch::outbound::resolve::ownership::safe_root::TrustedOwners;
    use accelerator::launch::start_up::{start_up, StartUp};
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

        fn re_resolve(
            &self,
            _command: &ExternalCommand,
            _missing: &Path,
        ) -> ReResolution {
            ReResolution::Declined
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
    fn an_integrity_failure_exits_one_when_non_blocking_is_forwarded() {
        let error = dispatch_error(integrity_failure);
        for tokens in [
            vec!["--non-blocking"],
            vec!["--fail-safe", "--non-blocking"],
        ] {
            let command = Command::External(
                tokens.into_iter().map(OsString::from).collect(),
            );
            assert_eq!(
                handle_dispatch_error(&error, &command),
                ExitCode::FAILURE
            );
        }
    }

    #[test]
    fn an_availability_failure_exits_failure_when_only_non_blocking() {
        let error = dispatch_error(availability_failure);
        let command = Command::External(vec![OsString::from("--non-blocking")]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::FAILURE);
    }

    #[test]
    fn an_integrity_failure_exits_two_without_fail_safe() {
        let error = dispatch_error(integrity_failure);
        let command = Command::External(vec![]);
        assert_eq!(handle_dispatch_error(&error, &command), ExitCode::from(2));
    }

    static ENVIRONMENT: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct ScopedVariable {
        name: &'static str,
        previous: Option<OsString>,
    }

    impl ScopedVariable {
        fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            Self { name, previous }
        }

        fn unset(name: &'static str) -> Self {
            let previous = std::env::var_os(name);
            std::env::remove_var(name);
            Self { name, previous }
        }
    }

    impl Drop for ScopedVariable {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    fn production_start_up() -> StartUp {
        start_up(
            &CacheRootConfig::from_env(config_adapters::plugin_root_from_env()),
            TrustedOwners::none(),
            super::production_ports(),
        )
    }

    fn frobnicate() -> ExternalCommand {
        ExternalCommand {
            name: OsString::from("frobnicate"),
            args: vec![],
        }
    }

    #[test]
    fn an_override_path_is_never_re_resolved() {
        let _serial = ENVIRONMENT.lock();
        let _override =
            ScopedVariable::set("ACCELERATOR_FROBNICATE_BIN", "/absent/tool");

        let start = production_start_up();
        let outcome = super::LazyProductionResolver {
            start: &start,
            consumed: &[],
        }
        .re_resolve(&frobnicate(), Path::new("/absent/tool"));

        assert!(matches!(outcome, ReResolution::Declined), "{outcome:?}");
    }

    #[cfg(unix)]
    #[test]
    fn an_unwritable_cache_root_is_re_resolved_through_the_cold_path(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt as _;

        let _serial = ENVIRONMENT.lock();
        let root = tempfile::Builder::new().prefix("re-resolve-").tempdir()?;
        let read_only = root.path().join("cache");
        std::fs::create_dir(&read_only)?;
        std::fs::set_permissions(
            &read_only,
            std::fs::Permissions::from_mode(0o555),
        )?;
        let _no_override = ScopedVariable::unset("ACCELERATOR_FROBNICATE_BIN");
        let _cache = ScopedVariable::set("ACCELERATOR_CACHE_DIR", &read_only);

        let start = production_start_up();
        let outcome = super::LazyProductionResolver {
            start: &start,
            consumed: &[],
        }
        .re_resolve(&frobnicate(), &read_only.join("frobnicate-entry"));

        std::fs::set_permissions(
            &read_only,
            std::fs::Permissions::from_mode(0o755),
        )?;
        assert!(
            matches!(
                outcome,
                ReResolution::Failed(
                    ResolutionError::CacheRootUnavailable { .. }
                )
            ),
            "{outcome:?}"
        );
        Ok(())
    }

    #[cfg(feature = "test-loopback")]
    #[allow(clippy::expect_used)]
    mod test_release {
        use std::path::{Path, PathBuf};

        use accelerator::launch::outbound::resolve::fetcher::FetchError;

        use super::super::{
            release_fetcher_for, release_trust_for, ReleaseTrust,
        };

        const LOOPBACK: &str = "http://127.0.0.1:8123/releases";

        struct KeyFile {
            _directory: tempfile::TempDir,
            path: PathBuf,
        }

        impl KeyFile {
            fn path(&self) -> &Path {
                &self.path
            }
        }

        fn key_file() -> KeyFile {
            let directory = tempfile::tempdir().expect("key directory");
            let path = directory.path().join("release.pub");
            std::fs::write(&path, "untrusted comment: test\nKEY\n")
                .expect("key contents");
            KeyFile {
                _directory: directory,
                path,
            }
        }

        #[test]
        fn a_loopback_base_url_trusts_the_test_key() {
            let key = key_file();

            assert_eq!(
                release_trust_for(LOOPBACK, Some(key.path())),
                ReleaseTrust::TestKey {
                    path: key.path().to_path_buf(),
                    contents: "untrusted comment: test\nKEY\n".to_owned(),
                }
            );
        }

        #[test]
        fn a_non_loopback_base_url_ignores_the_test_key() {
            let key = key_file();

            for base_url in [
                "https://github.com/atomicinnovation/accelerator/releases",
                "https://127.0.0.1:8123",
                "http://localhost:8123",
                "http://10.0.0.1:8123",
            ] {
                assert_eq!(
                    release_trust_for(base_url, Some(key.path())),
                    ReleaseTrust::Embedded,
                    "{base_url}"
                );
            }
        }

        #[test]
        fn a_lookalike_host_is_not_loopback() {
            let key = key_file();

            for base_url in
                ["http://127.0.0.1.example", "http://127.0.0.1@example"]
            {
                assert_eq!(
                    release_trust_for(base_url, Some(key.path())),
                    ReleaseTrust::Embedded,
                    "{base_url}"
                );
            }
        }

        #[test]
        fn a_missing_key_file_falls_back_to_the_embedded_key() {
            assert_eq!(
                release_trust_for(LOOPBACK, Some(Path::new("/absent/key.pub"))),
                ReleaseTrust::Embedded
            );
            assert_eq!(
                release_trust_for(LOOPBACK, None),
                ReleaseTrust::Embedded
            );
        }

        #[test]
        fn a_non_loopback_base_url_keeps_https_only() {
            for base_url in [
                "https://github.com/atomicinnovation/accelerator/releases",
                "http://127.0.0.1.example",
                "http://127.0.0.1@example",
            ] {
                let refused = release_fetcher_for(base_url)
                    .expect("fetcher")
                    .get("http://127.0.0.1:1/manifest.json")
                    .expect_err("an http fetch");

                assert!(
                    matches!(
                        &refused,
                        FetchError::Unreachable(detail)
                            if detail.contains("refusing non-https")
                    ),
                    "{base_url}: {refused:?}"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_block_views_read_the_personal_level_through_the_screen(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt as _;

        use config::Level;

        let dir = tempfile::Builder::new()
            .prefix("compose-stack-")
            .tempdir()?;
        std::fs::create_dir_all(dir.path().join(".git"))?;
        std::fs::create_dir_all(dir.path().join(".accelerator"))?;
        let personal = dir.path().join(".accelerator/config.local.md");
        std::fs::write(&personal, "---\npaths:\n  work: x\n---\nbody\n")?;
        std::fs::set_permissions(
            &personal,
            std::fs::Permissions::from_mode(0o644),
        )?;

        let stack = super::compose_stack(
            config_adapters::LegacyPolicy::Reject,
            Some(dir.path().to_path_buf()),
            super::PersonalFileWarning::Direct,
        )?;

        assert_eq!(stack.levels().read(Level::Personal)?, None);
        assert_eq!(stack.content().config_body(Level::Personal)?, None);
        Ok(())
    }
}
