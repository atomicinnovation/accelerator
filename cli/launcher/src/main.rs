//! The accelerator launcher binary — the composition root: it installs the TLS
//! crypto provider, initialises logging, wires the concrete adapters to the
//! ports, parses the CLI, and dispatches (built-ins in-process, external
//! subcommands via resolve + exec).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser as _};

use accelerator::launch::core::{
    ExternalCommand, ResolutionError, ResolveBinary,
};
use accelerator::launch::dispatch;
use accelerator::launch::help::external_subcommands_section;
use accelerator::launch::inbound::cli::Cli;
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
/// touch the cache root, TLS, or the network.
struct LazyProductionResolver;

impl ResolveBinary for LazyProductionResolver {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        if let Some(path) = override_path(&command.name)? {
            return Ok(path);
        }
        let cache = cache_root::resolve(&CacheRootConfig::from_env())?;
        let keys = TrustedKeys::embedded()?;
        let config = ResolverConfig::production(release_base_url(), cache);
        FetchVerifyCacheResolver::new(config, keys)?.resolve(command)
    }
}

/// The external-subcommands help section, or `None` on any failure so `--help`
/// still prints the built-in help. Reads only the manifest, no cache root.
fn help_section() -> Option<String> {
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

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let reporter = VersionReporter::new(VergenBuildMetadata);
    let resolver = LazyProductionResolver;
    let executor = UnixExec;
    dispatch(cli, &reporter, &resolver, &executor)
}

fn main() -> ExitCode {
    if let Err(error) = install_crypto_provider() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    // try_parse so top-level `--help` can be intercepted and augmented; a
    // `foo --help` routes to External and is delegated to the child.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            return render_augmented_help();
        }
        Err(error) => error.exit(),
    };

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
