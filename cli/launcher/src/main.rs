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
use accelerator::launch::core::{
    ExternalCommand, ResolutionError, ResolveBinary,
};
use accelerator::launch::dispatch;
use accelerator::launch::help::external_subcommands_section;
use accelerator::launch::inbound::cli::{Cli, Command};
use accelerator::launch::outbound::exec::UnixExec;
use accelerator::launch::outbound::override_path;
use accelerator::launch::outbound::resolve::cache_root::{
    self, CacheRootConfig,
};
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::{
    FetchVerifyCacheResolver, ResolverConfig,
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
        let cache = cache_root::resolve(&CacheRootConfig::from_env())?;
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
        Some("version" | "config" | "help")
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
        Command::Version | Command::External(_) => LegacyPolicy::Reject,
    }
}

/// Composes the `config` port bundle at the current directory's project root,
/// applying the resolved legacy policy. Invoked lazily by `dispatch`.
fn compose_stack(policy: LegacyPolicy) -> Result<ConfigStack, ConfigError> {
    let cwd = std::env::current_dir().map_err(|error| ConfigError::Io {
        path: ".".to_owned(),
        detail: error.to_string(),
    })?;
    let composed = config_adapters::compose(&cwd, policy)?;
    Ok(ConfigStack::new(
        Box::new(composed.service),
        Box::new(composed.store),
    ))
}

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let reporter = VersionReporter::new(VergenBuildMetadata);
    let resolver = LazyProductionResolver;
    let executor = UnixExec;
    let policy = legacy_policy(&cli.command);
    dispatch(cli, &reporter, &resolver, &executor, move || {
        compose_stack(policy)
    })
}

fn report(error: &kernel::Error) -> ExitCode {
    eprintln!("{error}");
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
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
        Err(error) => report(&error),
    }
}
