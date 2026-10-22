//! Hermetic tests of the fetch → verify → cache resolver, in-process against a
//! local mock server with an injected, freshly-generated signing key. Requires
//! the `minisign` CLI; skips cleanly if it is absent.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use std::cell::RefCell;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use http_test_support::{MockHTTPServer, RequestKey, Route};
use tempfile::TempDir;

use accelerator::launch::core::{
    run_external, ExecBinary, ExternalCommand, ReResolution, ResolutionError,
    ResolveBinary,
};
use accelerator::launch::outbound::resolve::cache_root::{
    probe_attempts, CacheRootConfig,
};
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::filesystem::{
    CacheFilesystem, SystemFilesystem,
};
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::ownership::safe_root::TrustedOwners;
use accelerator::launch::outbound::resolve::ownership::ProcessOwnership;
use accelerator::launch::outbound::resolve::prune_report::NoPruneSteps;
use accelerator::launch::outbound::resolve::retention::launcher_id;
use accelerator::launch::outbound::resolve::retention::ports::CachePorts;
use accelerator::launch::outbound::resolve::retention::residue::KillZeroProbe;
use accelerator::launch::outbound::resolve::retention_windows::{
    CLAIM_WINDOW, REFRESH_FRACTION,
};
use accelerator::launch::outbound::resolve::tree::lease::FlockProbe;
use accelerator::launch::outbound::resolve::tree::locks::FlockLocks;
use accelerator::launch::outbound::resolve::tree::reap::UnobservedRemovals;
use accelerator::launch::outbound::resolve::tree::{
    ExpectedDigests, SystemClock,
};
use accelerator::launch::outbound::resolve::verifier::sha256_hex;
use accelerator::launch::outbound::resolve::{
    cache, FetchVerifyCacheResolver, ResolverConfig, HOST_PLATFORM,
};
use accelerator::launch::start_up::{start_up, DispatchConfig, StartUp};
use common::recording_filesystem::{
    in_root, outside, FilesystemCall, RecordingFilesystem,
};
use common::signed_release::{
    asset_path, generate_keypair, manifest_json, sign, SignedRelease, BINARY,
    VERSION,
};


fn tempdir(tag: &str) -> TempDir {
    tempfile::Builder::new()
        .prefix(&format!("res-{tag}-"))
        .tempdir()
        .expect("mkdir temp")
}

struct Harness {
    server: MockHTTPServer,
    cache: PathBuf,
    trusted: Vec<String>,
    fixture_bytes: Vec<u8>,
    sha: String,
    asset_sig: String,
    workdir: PathBuf,
    plugin_root_of_cache: PathBuf,
    minisign: PathBuf,
    trusted_secret: PathBuf,
    // RAII guards: keep the temp dirs alive for the harness's lifetime and
    // remove them on drop.
    _workdir_guard: TempDir,
    _cache_guard: TempDir,
}

impl Harness {
    fn config(&self, base_url: String) -> ResolverConfig {
        ResolverConfig {
            expected_version: VERSION.to_owned(),
            platform: HOST_PLATFORM.to_owned(),
            base_url,
            cache_root: self.cache.clone(),
        }
    }

    fn keys(&self) -> TrustedKeys {
        TrustedKeys::from_public_key_files(
            &self.trusted.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .expect("trusted keys")
    }

    fn resolver_for(&self, base_url: String) -> FetchVerifyCacheResolver {
        self.resolver_over(base_url, Arc::new(SystemFilesystem))
    }

    fn resolver_over(
        &self,
        base_url: String,
        filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
    ) -> FetchVerifyCacheResolver {
        FetchVerifyCacheResolver::with_fetcher(
            self.config(base_url),
            self.keys(),
            Fetcher::with_backoff(std::time::Duration::from_millis(1))
                .expect("fetcher"),
            filesystem,
        )
    }

    fn recorder(&self) -> RecordingFilesystem {
        RecordingFilesystem::new(&self.cache, &self.workdir)
    }

    fn resolve_recorded(
        &self,
        recorder: &RecordingFilesystem,
    ) -> Result<PathBuf, ResolutionError> {
        self.resolver_over(self.server.base_url(), Arc::new(recorder.clone()))
            .resolve(&ExternalCommand {
                name: OsString::from(BINARY),
                args: vec![],
            })
    }

    fn resolver(&self) -> FetchVerifyCacheResolver {
        self.resolver_for(self.server.base_url())
    }

    fn resolve(&self) -> Result<PathBuf, ResolutionError> {
        self.resolver().resolve(&ExternalCommand {
            name: OsString::from(BINARY),
            args: vec![],
        })
    }

    fn resolve_offline(&self) -> Result<PathBuf, ResolutionError> {
        self.resolver_for("http://127.0.0.1:1".to_owned()).resolve(
            &ExternalCommand {
                name: OsString::from(BINARY),
                args: vec![],
            },
        )
    }

    /// The cache is the plugin's own `bin`, named with no override.
    fn plugin_bin_root(&self) -> CacheRootConfig {
        CacheRootConfig {
            cache_dir_override: None,
            plugin_root: Some(self.plugin_root_of_cache.clone()),
        }
    }

    /// The cache is an override naming a directory other than the plugin
    /// `bin`, which exists.
    fn relocated_root(&self) -> CacheRootConfig {
        let plugin_root = self.workdir.join("plugin");
        std::fs::create_dir_all(plugin_root.join("bin")).expect("plugin bin");
        CacheRootConfig {
            cache_dir_override: Some(self.cache.clone()),
            plugin_root: Some(plugin_root),
        }
    }

    fn dispatch_resolver(&self, start: &StartUp) -> FetchVerifyCacheResolver {
        start
            .dispatch_resolver(DispatchConfig {
                base_url: self.server.base_url(),
                keys: self.keys(),
                fetcher: Fetcher::with_backoff(
                    std::time::Duration::from_millis(1),
                )
                .expect("fetcher"),
            })
            .expect("dispatch resolver")
    }

    /// Start up and resolve as `run` does, recording every call.
    fn dispatch_recorded(
        &self,
        config: &CacheRootConfig,
        recorder: &RecordingFilesystem,
    ) -> Result<PathBuf, ResolutionError> {
        let start = start_up_over(config, Arc::new(recorder.clone()));
        self.dispatch_resolver(&start).resolve(&visualiser())
    }

    fn version_claim(&self) -> PathBuf {
        self.cache
            .join("claims")
            .join(format!("{VERSION}.{}", launcher_id()))
    }

    fn seed_version_claim(&self, bytes: &[u8], modified: u64) {
        let claim = self.version_claim();
        std::fs::create_dir_all(claim.parent().expect("claims dir"))
            .expect("claims dir");
        std::fs::write(&claim, bytes).expect("claim");
        SystemFilesystem
            .set_modified(&claim, modified)
            .expect("claim mtime");
    }

    /// Write a verifiable cache entry directly, without resolving — so a
    /// warm-path test starts with no probe of its own already counted.
    #[track_caller]
    fn seed_cache(&self) -> Result<cache::CachedBinary, Box<dyn Error>> {
        let cached = cache::store(
            &SystemFilesystem,
            &self.cache,
            BINARY,
            VERSION,
            &self.sha,
            &self.fixture_bytes,
            &self.asset_sig,
        )?;
        assert!(
            cache::find(&SystemFilesystem, &self.cache, BINARY, VERSION)
                .is_some(),
            "a seeded entry must be findable, or the warm-path tests silently \
             degrade into cold-miss duplicates"
        );
        Ok(cached)
    }

    /// Replace the cache directory rather than unlinking entries from a live
    /// `read_dir` stream, whose behaviour under concurrent removal POSIX
    /// leaves unspecified.
    #[track_caller]
    fn clear_cache(&self) -> Result<(), Box<dyn Error>> {
        std::fs::remove_dir_all(&self.cache)?;
        std::fs::create_dir_all(&self.cache)?;
        assert!(
            cache::find(&SystemFilesystem, &self.cache, BINARY, VERSION)
                .is_none(),
            "the cache must be empty, or the next resolution is a hit"
        );
        Ok(())
    }
}

fn start_up_over(
    config: &CacheRootConfig,
    filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
) -> StartUp {
    start_up(
        config,
        TrustedOwners::none(),
        Arc::new(CachePorts {
            filesystem,
            ownership: Arc::new(ProcessOwnership::default()),
            clock: Arc::new(SystemClock),
            locks: Arc::new(FlockLocks),
            steps: Arc::new(NoPruneSteps),
            bootstrap_holders: Arc::new(KillZeroProbe),
            lease_liveness: Arc::new(FlockProbe),
            pins: Arc::new(ExpectedDigests::Compiled),
            removals: Arc::new(UnobservedRemovals),
        }),
    )
}

#[track_caller]
fn probes_during<T>(
    branch: &str,
    expected: u64,
    action: impl FnOnce() -> T,
) -> T {
    let before = probe_attempts();
    let result = action();
    assert_eq!(
        probe_attempts() - before,
        expected,
        "{branch}: expected {expected} probe attempt(s)"
    );
    result
}

/// Build a harness with a correctly-signed release the resolver will accept.
fn happy_harness() -> Option<Harness> {
    let workdir_guard = tempdir("work");
    let workdir = workdir_guard.path().to_path_buf();
    let release = SignedRelease::serve(&workdir)?;
    let cache_guard = tempdir("cache");
    let plugin_root_of_cache = cache_guard.path().to_path_buf();
    let cache = plugin_root_of_cache.join("bin");
    std::fs::create_dir(&cache).expect("mkdir cache");

    Some(Harness {
        server: release.server,
        cache,
        trusted: release.trusted,
        fixture_bytes: release.fixture_bytes,
        sha: release.sha,
        asset_sig: release.asset_sig,
        workdir,
        plugin_root_of_cache,
        minisign: release.minisign,
        trusted_secret: release.trusted_secret,
        _workdir_guard: workdir_guard,
        _cache_guard: cache_guard,
    })
}

macro_rules! skip_if_no_minisign {
    ($harness:expr) => {
        match $harness {
            Some(harness) => harness,
            // Fail closed under CI rather than returning a false green: these
            // cases cover the signature path, and `minisign` is pinned in
            // `mise.toml`, so an absent binary in CI is a misconfiguration, not
            // a reason to skip. Locally it still skips cleanly.
            None if std::env::var_os("CI").is_some() => {
                panic!("minisign is required under CI and was not on PATH")
            }
            None => {
                eprintln!("skipping: minisign not on PATH");
                return Ok(());
            }
        }
    };
}

fn default_root_warm_dispatch_calls(sha: &str) -> Vec<FilesystemCall> {
    let entry = in_root(format!("{BINARY}-{VERSION}-{sha}"));
    let signature = in_root(format!("{BINARY}-{VERSION}-{sha}.minisig"));
    vec![
        FilesystemCall::List(in_root("")),
        FilesystemCall::Exists(signature.clone()),
        FilesystemCall::Read(entry),
        FilesystemCall::ReadToString(signature),
    ]
}

#[test]
fn a_default_root_warm_dispatch_performs_exactly_the_fixed_calls(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    let recorder = harness.recorder();

    harness.resolve_recorded(&recorder)?;

    assert_eq!(
        recorder.calls(),
        default_root_warm_dispatch_calls(&harness.sha)
    );
    Ok(())
}

struct RemovingAfterResolve(FetchVerifyCacheResolver);

impl ResolveBinary for RemovingAfterResolve {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        let path = self.0.resolve(command)?;
        let mut signature = path.clone().into_os_string();
        signature.push(".minisig");
        std::fs::remove_file(&path).expect("remove the resolved entry");
        std::fs::remove_file(signature).expect("remove its signature");
        Ok(path)
    }

    fn re_resolve(
        &self,
        command: &ExternalCommand,
        missing: &Path,
    ) -> ReResolution {
        self.0.re_resolve(command, missing)
    }
}

/// Fails as `exec` does on a missing program, and otherwise records the
/// program as executed.
#[derive(Default)]
struct ExecUnlessMissing {
    executed: RefCell<Vec<PathBuf>>,
}

impl ExecBinary for ExecUnlessMissing {
    fn exec(&self, program: &Path, _args: &[OsString]) -> ResolutionError {
        if !program.exists() {
            return ResolutionError::Exec {
                program: program.to_path_buf(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            };
        }
        self.executed.borrow_mut().push(program.to_path_buf());
        ResolutionError::Exec {
            program: program.to_path_buf(),
            source: std::io::Error::other("executed"),
        }
    }
}

/// Fails as `exec` does when a present binary's interpreter is missing.
struct ExecMissingInterpreter;

impl ExecBinary for ExecMissingInterpreter {
    fn exec(&self, program: &Path, _args: &[OsString]) -> ResolutionError {
        ResolutionError::Exec {
            program: program.to_path_buf(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        }
    }
}

fn visualiser() -> ExternalCommand {
    ExternalCommand {
        name: OsString::from(BINARY),
        args: vec![],
    }
}

#[derive(Debug, Clone, Copy)]
enum RootKind {
    PluginBin,
    Relocated,
}

const BOTH_ROOT_KINDS: [RootKind; 2] =
    [RootKind::PluginBin, RootKind::Relocated];

impl Harness {
    fn root_of(&self, kind: RootKind) -> CacheRootConfig {
        match kind {
            RootKind::PluginBin => self.plugin_bin_root(),
            RootKind::Relocated => self.relocated_root(),
        }
    }
}

#[test]
fn an_entry_removed_between_find_and_exec_is_refetched_once(
) -> Result<(), Box<dyn Error>> {
    for kind in BOTH_ROOT_KINDS {
        let harness = skip_if_no_minisign!(happy_harness());
        let seeded = harness.seed_cache()?;
        let start =
            start_up_over(&harness.root_of(kind), Arc::new(SystemFilesystem));
        let resolver = RemovingAfterResolve(harness.dispatch_resolver(&start));
        let executor = ExecUnlessMissing::default();

        run_external(&resolver, &executor, &visualiser());

        let executed: Vec<PathBuf> = executor
            .executed
            .borrow()
            .iter()
            .map(|path| path.canonicalize().expect("executed path"))
            .collect();
        assert_eq!(executed, [seeded.path.canonicalize()?], "{kind:?}");
        assert_eq!(
            std::fs::read(&seeded.path)?,
            harness.fixture_bytes,
            "{kind:?}"
        );
        assert_eq!(
            harness.server.hits(&RequestKey::get(&asset_path())),
            1,
            "{kind:?}"
        );
    }
    Ok(())
}

#[test]
fn an_existing_but_unrunnable_entry_is_not_refetched(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;

    let error = run_external(
        &harness.resolver(),
        &ExecMissingInterpreter,
        &visualiser(),
    );

    assert!(
        matches!(
            &error,
            ResolutionError::Exec { program, .. } if *program == seeded.path
        ),
        "{error:?}"
    );
    assert_eq!(harness.server.hits(&RequestKey::get(&asset_path())), 0);
    Ok(())
}

#[test]
fn happy_path_fetches_verifies_caches_and_returns_a_runnable_binary(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let path = harness.resolve()?;
    assert!(path.exists(), "cached binary missing");
    assert_eq!(std::fs::read(&path)?, harness.fixture_bytes);
    let status = Command::new(&path).arg("exit-42").status()?;
    assert_eq!(status.code(), Some(42));
    Ok(())
}

#[test]
fn cache_reuse_does_not_refetch() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.resolve()?;
    harness.resolve()?;
    assert_eq!(
        harness.server.hits(&RequestKey::get(&asset_path())),
        1,
        "second resolve must reuse the cache, not refetch"
    );
    Ok(())
}

#[test]
fn a_checksum_mismatch_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // sha256 wrong, but the manifest is still validly signed.
    let wrong_sha = "a".repeat(64);
    let asset_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        &harness.fixture_bytes,
    );
    let manifest = manifest_json(VERSION, &wrong_sha, &asset_sig);
    let manifest_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        manifest.as_bytes(),
    );
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Bytes {
            status: 200,
            body: manifest.into_bytes(),
        },
    );
    harness.server.route(
        RequestKey::get("/manifest.minisig"),
        Route::Bytes {
            status: 200,
            body: manifest_sig.into_bytes(),
        },
    );

    let error = harness
        .resolve()
        .err()
        .ok_or("expected a checksum mismatch")?;
    assert!(matches!(error, ResolutionError::ChecksumMismatch { .. }));
    // `BINARY` ("visualiser") is the only other `DISPATCHED_SUBBINARIES`
    // entry besides `vcs`: this pins that its exit code deliberately changes
    // from 1 (Failed) to 2 (Refusal) on an integrity failure, uniformly
    // across every dispatched sub-binary, not just `vcs`.
    let kernel_error: kernel::Error = error.into();
    assert!(matches!(kernel_error, kernel::Error::Refusal(_)));
    Ok(())
}

#[test]
fn a_non_release_key_signature_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // Attacker-signed asset with a correct sha256: verification is key-bound.
    let (_attacker_pub, attacker_secret) =
        generate_keypair(&harness.minisign, &harness.workdir, "attacker");
    let sha = sha256_hex(&harness.fixture_bytes);
    let asset_sig = sign(
        &harness.minisign,
        &attacker_secret,
        &harness.workdir,
        &harness.fixture_bytes,
    );
    // The manifest stays trusted-signed so it passes before the asset check.
    let manifest = manifest_json(VERSION, &sha, &asset_sig);
    let manifest_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        manifest.as_bytes(),
    );
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Bytes {
            status: 200,
            body: manifest.into_bytes(),
        },
    );
    harness.server.route(
        RequestKey::get("/manifest.minisig"),
        Route::Bytes {
            status: 200,
            body: manifest_sig.into_bytes(),
        },
    );

    let error = harness
        .resolve()
        .err()
        .ok_or("expected a signature mismatch")?;
    assert!(matches!(error, ResolutionError::SignatureMismatch { .. }));
    let kernel_error: kernel::Error = error.into();
    assert!(matches!(kernel_error, kernel::Error::Refusal(_)));
    Ok(())
}

#[test]
fn a_tampered_manifest_signature_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // Manifest bytes differ from what manifest.minisig signs.
    let tampered = manifest_json(VERSION, &"a".repeat(64), "junk");
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Bytes {
            status: 200,
            body: tampered.into_bytes(),
        },
    );
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::ManifestSignature)
    ));
    Ok(())
}

#[test]
fn a_wrong_version_manifest_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let sha = sha256_hex(&harness.fixture_bytes);
    let asset_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        &harness.fixture_bytes,
    );
    let manifest = manifest_json("9.9.9", &sha, &asset_sig);
    let manifest_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        manifest.as_bytes(),
    );
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Bytes {
            status: 200,
            body: manifest.into_bytes(),
        },
    );
    harness.server.route(
        RequestKey::get("/manifest.minisig"),
        Route::Bytes {
            status: 200,
            body: manifest_sig.into_bytes(),
        },
    );

    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::ManifestVersionMismatch { .. })
    ));
    Ok(())
}

#[test]
fn an_unsupported_higher_schema_is_refused_with_the_schema_error(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // schema_version 2 is gated before the version check, so the error names
    // the schema, not a version mismatch.
    let sha = sha256_hex(&harness.fixture_bytes);
    let asset_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        &harness.fixture_bytes,
    );
    let escaped = asset_sig.replace('\n', "\\n").replace('\t', "\\t");
    let manifest = format!(
        "{{\"schema_version\":2,\"version\":\"{VERSION}\",\"binaries\":{{\
         \"{BINARY}\":{{\"description\":\"Visualiser\",\"platforms\":{{\
         \"{HOST_PLATFORM}\":{{\"sha256\":\"{sha}\",\"signature\":\"{escaped}\"\
         }}}}}}}}}}"
    );
    let manifest_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        manifest.as_bytes(),
    );
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Bytes {
            status: 200,
            body: manifest.into_bytes(),
        },
    );
    harness.server.route(
        RequestKey::get("/manifest.minisig"),
        Route::Bytes {
            status: 200,
            body: manifest_sig.into_bytes(),
        },
    );

    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::UnsupportedSchema { .. })
    ));
    Ok(())
}

#[test]
fn a_missing_release_is_named_and_execs_nothing() -> Result<(), Box<dyn Error>>
{
    let harness = skip_if_no_minisign!(happy_harness());
    harness
        .server
        .route(RequestKey::get("/manifest.json"), Route::Status(404));
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::ReleaseUnavailable { .. })
    ));
    Ok(())
}

#[test]
fn a_missing_asset_is_named() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness
        .server
        .route(RequestKey::get(&asset_path()), Route::Status(404));
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::AssetNotFound { .. })
    ));
    Ok(())
}

#[test]
fn a_persistent_server_error_gives_up_after_bounded_retries(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness
        .server
        .route(RequestKey::get("/manifest.json"), Route::Status(500));
    // Exhausted 5xx retries map to Fetch (a 404 would be ReleaseUnavailable).
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::Fetch { .. })
    ));
    assert_eq!(
        harness.server.hits(&RequestKey::get("/manifest.json")),
        3,
        "bounded retries"
    );
    Ok(())
}

#[test]
fn a_transient_5xx_recovers_within_the_retry_budget(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // Fail the asset fetch twice then succeed; its signature is already inline.
    harness.server.route(
        RequestKey::get(&asset_path()),
        Route::FlakyThenOk {
            fail_times: 2,
            body: harness.fixture_bytes.clone(),
        },
    );
    let path = harness.resolve()?;
    assert!(path.exists());
    assert_eq!(harness.server.hits(&RequestKey::get(&asset_path())), 3);
    Ok(())
}

#[test]
fn a_redirect_to_a_disallowed_host_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // 127.0.0.1 is off the redirect allowlist; the unfollowed 302 is a transport
    // failure (Fetch), not a 404 (ReleaseUnavailable).
    harness.server.route(
        RequestKey::get("/manifest.json"),
        Route::Redirect {
            status: 302,
            location: format!("{}/elsewhere", harness.server.base_url()),
        },
    );
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::Fetch { .. })
    ));
    Ok(())
}

#[test]
fn an_already_cached_binary_resolves_offline() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let first = harness.resolve()?;
    assert!(first.exists());
    let path = harness.resolve_offline()?;
    assert_eq!(path, first);
    Ok(())
}

#[test]
fn a_poisoned_cache_entry_is_replaced_in_place_and_reexecs(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let path = harness.resolve()?;
    std::fs::write(&path, b"poisoned")?;
    let healed = harness.resolve()?;
    assert_eq!(healed, path, "the same cache path is replaced in place");
    assert_eq!(std::fs::read(&healed)?, harness.fixture_bytes, "refetched");
    assert!(
        harness.server.hits(&RequestKey::get(&asset_path())) >= 2,
        "a refetch must have occurred"
    );
    Ok(())
}

#[test]
fn a_corrupt_entry_is_replaced_by_a_rename_with_no_unlink_before_it(
) -> Result<(), Box<dyn Error>> {
    for kind in BOTH_ROOT_KINDS {
        let harness = skip_if_no_minisign!(happy_harness());
        let seeded = harness.seed_cache()?;
        std::fs::write(&seeded.path, b"poisoned")?;
        let recorder = harness.recorder();

        harness.dispatch_recorded(&harness.root_of(kind), &recorder)?;

        let mutations: Vec<FilesystemCall> = recorder
            .calls_touching(&seeded.path)
            .into_iter()
            .filter(FilesystemCall::is_mutation)
            .collect();
        assert!(
            matches!(
                mutations.as_slice(),
                [FilesystemCall::Rename { to, .. }]
                    if *to == recorder.recorded(&seeded.path)
            ),
            "{kind:?}: {mutations:?}"
        );
        assert_eq!(
            std::fs::read(&seeded.path)?,
            harness.fixture_bytes,
            "{kind:?}"
        );
    }
    Ok(())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after epoch")
        .as_secs()
}

const REFRESH_AGE: u64 = CLAIM_WINDOW.as_secs() / REFRESH_FRACTION;

#[test]
fn an_override_root_dispatch_writes_its_claim_before_find(
) -> Result<(), Box<dyn Error>> {
    for seeded_claim_age in [None, Some(REFRESH_AGE + 60)] {
        let harness = skip_if_no_minisign!(happy_harness());
        harness.seed_cache()?;
        if let Some(age) = seeded_claim_age {
            harness.seed_version_claim(b"", now() - age);
        }
        let recorder = harness.recorder();

        harness.dispatch_recorded(&harness.relocated_root(), &recorder)?;

        let calls = recorder.calls();
        let claim_written = calls.iter().position(|call| {
            *call
                == FilesystemCall::WritePrivate(
                    recorder.recorded(&harness.version_claim()),
                )
        });
        let found = calls
            .iter()
            .position(|call| *call == FilesystemCall::List(in_root("")));
        assert!(
            claim_written
                .zip(found)
                .is_some_and(|(claim, find)| claim < find),
            "{seeded_claim_age:?}: {calls:?}"
        );
    }
    Ok(())
}

#[test]
fn an_override_root_dispatch_with_a_young_claim_does_not_rewrite_it(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    harness.seed_version_claim(b"sentinel", now());
    let recorder = harness.recorder();

    harness.dispatch_recorded(&harness.relocated_root(), &recorder)?;

    assert!(recorder
        .calls_touching(&harness.version_claim())
        .iter()
        .all(|call| !call.is_mutation()));
    assert_eq!(std::fs::read(harness.version_claim())?, b"sentinel");
    Ok(())
}

#[test]
fn a_fresh_claim_costs_one_metadata_read_beyond_the_fixed_list(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    harness.seed_version_claim(b"", now());
    let recorder = harness.recorder();

    harness.dispatch_recorded(&harness.relocated_root(), &recorder)?;

    let mut expected = vec![
        FilesystemCall::Resolve(in_root("")),
        FilesystemCall::Resolve(outside("plugin/bin")),
        FilesystemCall::Entry(recorder.recorded(&harness.version_claim())),
    ];
    expected.extend(default_root_warm_dispatch_calls(&harness.sha));
    assert_eq!(recorder.calls(), expected);
    Ok(())
}

#[test]
fn naming_the_plugin_bin_adds_only_the_canonical_resolution(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    let recorder = harness.recorder();
    let config = CacheRootConfig {
        cache_dir_override: Some(harness.cache.clone()),
        ..harness.plugin_bin_root()
    };

    harness.dispatch_recorded(&config, &recorder)?;

    let mut expected = vec![
        FilesystemCall::Resolve(in_root("")),
        FilesystemCall::Resolve(in_root("")),
    ];
    expected.extend(default_root_warm_dispatch_calls(&harness.sha));
    assert_eq!(recorder.calls(), expected);
    Ok(())
}

#[test]
fn an_unset_override_performs_exactly_the_fixed_calls(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    let recorder = harness.recorder();

    harness.dispatch_recorded(&harness.plugin_bin_root(), &recorder)?;

    assert_eq!(
        recorder.calls(),
        default_root_warm_dispatch_calls(&harness.sha)
    );
    Ok(())
}

#[test]
fn a_read_only_override_root_still_dispatches_warm(
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt as _;
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    let config = harness.relocated_root();
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o555),
    )?;

    let result = harness
        .dispatch_recorded(&config, &harness.recorder())
        .map(|path| path.canonicalize());

    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert_eq!(result??, seeded.path.canonicalize()?);
    assert!(!harness.version_claim().exists());
    Ok(())
}

#[test]
fn a_development_checkout_relocated_elsewhere_claims_its_version(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    let checkout = harness.workdir.join("checkout");
    std::fs::create_dir_all(checkout.join("bin"))?;
    std::fs::write(checkout.join("bin/accelerator"), b"#!/bin/sh\n")?;
    let config = CacheRootConfig {
        cache_dir_override: Some(harness.cache.clone()),
        plugin_root: Some(checkout),
    };

    harness.dispatch_recorded(&config, &harness.recorder())?;

    assert!(harness.version_claim().is_file());
    Ok(())
}

#[test]
fn a_poisoned_cache_entry_offline_is_a_distinct_diagnostic(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let path = harness.resolve()?;
    std::fs::write(&path, b"poisoned")?;
    // No server to re-fetch from → the distinct corrupt-and-refetch diagnostic.
    assert!(matches!(
        harness.resolve_offline(),
        Err(ResolutionError::CorruptCacheAndRefetchFailed { .. })
    ));
    Ok(())
}

#[test]
fn a_signature_read_io_error_propagates_the_refetch_error_verbatim(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.resolve()?;
    let cached =
        cache::find(&SystemFilesystem, &harness.cache, BINARY, VERSION)
            .ok_or("cached entry missing")?;
    // Invalid UTF-8 in the signature sidecar: `fs::read_to_string` fails with
    // a plain Cache I/O error, distinct from a checksum/signature mismatch —
    // the cached binary bytes themselves are untouched.
    std::fs::write(&cached.signature_path, [0xFF, 0xFE, 0xFD])?;
    // The entry must still be findable, or resolution takes the cold-miss tail
    // call instead of the cache-I/O refetch arm and still yields `Fetch`.
    assert!(
        cache::find(&SystemFilesystem, &harness.cache, BINARY, VERSION)
            .is_some()
    );
    let result =
        probes_during("sidecar I/O refetch", 1, || harness.resolve_offline());
    assert!(
        matches!(result, Err(ResolutionError::Fetch { .. })),
        "a benign I/O hiccup's failed refetch must propagate the refetch's \
         own error verbatim, not CorruptCacheAndRefetchFailed: {result:?}"
    );
    Ok(())
}

#[test]
fn a_cold_miss_probes_the_cache_root_exactly_once() -> Result<(), Box<dyn Error>>
{
    let harness = skip_if_no_minisign!(happy_harness());
    probes_during("cold miss", 1, || harness.resolve())?;
    Ok(())
}

#[test]
fn a_warm_hit_never_probes_the_cache_root() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.seed_cache()?;
    probes_during("warm hit", 0, || harness.resolve())?;
    Ok(())
}

#[test]
fn a_successful_refetch_probes_the_cache_root_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.path, b"poisoned")?;
    let healed = probes_during("refetch (checksum)", 1, || harness.resolve())?;
    assert_eq!(
        std::fs::read(&healed)?,
        harness.fixture_bytes,
        "replace-in-place self-heal: the poisoned bytes must be replaced"
    );
    Ok(())
}

#[test]
fn a_failed_refetch_probes_the_cache_root_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.path, b"poisoned")?;
    let result =
        probes_during("refetch failed", 1, || harness.resolve_offline());
    assert!(matches!(
        result,
        Err(ResolutionError::CorruptCacheAndRefetchFailed { .. })
    ));
    Ok(())
}

#[test]
fn a_refetch_after_a_benign_cache_io_error_probes_exactly_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let seeded = harness.seed_cache()?;
    std::fs::write(&seeded.signature_path, [0xFF, 0xFE, 0xFD])?;
    probes_during("refetch (cache I/O)", 1, || harness.resolve())?;
    Ok(())
}

#[test]
fn each_of_two_cold_misses_probes_the_cache_root_once(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    probes_during("first cold miss", 1, || harness.resolve())?;
    harness.clear_cache()?;
    probes_during("second cold miss", 1, || harness.resolve())?;
    Ok(())
}

#[test]
fn resolve_succeeds_from_a_read_only_cache_root_on_a_hit(
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt as _;
    let harness = skip_if_no_minisign!(happy_harness());
    let first = harness.resolve()?;
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o555),
    )?;
    let result = harness.resolve();
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert_eq!(result?, first, "a cache hit must skip the write probe");
    Ok(())
}

#[test]
fn an_unwritable_cache_root_fails_fast_and_correctly_on_a_miss(
) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt as _;
    let harness = skip_if_no_minisign!(happy_harness());
    std::fs::create_dir_all(&harness.cache)?;
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o555),
    )?;
    // Asserted after the restore, never inside `probes_during`: a panic in the
    // permissions window would skip the chmod-back.
    let before = probe_attempts();
    let result = harness.resolve();
    let attempts = probe_attempts() - before;
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert!(
        matches!(result, Err(ResolutionError::CacheRootUnavailable { .. })),
        "the write probe must still guard the write path: {result:?}"
    );
    assert_eq!(
        attempts, 1,
        "a failing probe must not be retried inside the resolver"
    );
    assert_eq!(
        harness.server.hits(&RequestKey::get("/manifest.json")),
        0,
        "verify_writable must run before any network round trip"
    );
    Ok(())
}

#[test]
fn two_concurrent_first_use_resolves_both_succeed() -> Result<(), Box<dyn Error>>
{
    let harness = skip_if_no_minisign!(happy_harness());
    std::thread::scope(|scope| {
        let a = scope.spawn(|| harness.resolve());
        let b = scope.spawn(|| harness.resolve());
        let first = a.join().expect("thread a");
        let second = b.join().expect("thread b");
        assert!(first.is_ok(), "concurrent resolve A failed: {first:?}");
        assert!(second.is_ok(), "concurrent resolve B failed: {second:?}");
    });
    let path = harness.resolve()?;
    assert_eq!(std::fs::read(&path)?, harness.fixture_bytes);
    Ok(())
}
