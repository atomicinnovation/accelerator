//! The accelerator launcher binary — the composition root: it installs the TLS
//! crypto provider, initialises logging, wires the concrete adapters to the
//! ports, parses the CLI, and dispatches (built-ins in-process, external
//! subcommands via resolve + exec).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser as _;

use launcher::launch::core::{ExternalCommand, ResolutionError, ResolveBinary};
use launcher::launch::dispatch;
use launcher::launch::inbound::cli::Cli;
use launcher::launch::outbound::exec::UnixExec;
use launcher::launch::outbound::override_path;
use launcher::launch::outbound::resolve::cache_root::{self, CacheRootConfig};
use launcher::launch::outbound::resolve::keys::TrustedKeys;
use launcher::launch::outbound::resolve::{
    FetchVerifyCacheResolver, ResolverConfig,
};
use launcher::launch::outbound::tls::install_crypto_provider;
use launcher::version::core::VersionReporter;
use launcher::version::outbound::build_metadata::VergenBuildMetadata;

/// The release-download base URL the real resolver fetches from, pinned to the
/// plugin's own `v{version}` tag. Overridable by `ACCELERATOR_RELEASE_BASE_URL`
/// (the hermetic tests point it at a local mock server).
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

/// Resolves external subcommands: the `ACCELERATOR_<SUB>_BIN` override first
/// (air-gapped escape hatch, no cache root or network needed), else the real
/// fetch → verify → cache resolver, built lazily so a built-in like `version`
/// never touches the cache root, TLS, or the network.
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
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
