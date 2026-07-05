//! Hermetic tests of the fetch → verify → cache resolver, in-process against a
//! local mock server with an injected, freshly-generated signing key. Requires
//! the `minisign` CLI; skips cleanly if it is absent.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use common::{MockServer, Route};

use accelerator::launch::core::{
    ExternalCommand, ResolutionError, ResolveBinary,
};
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::verifier::sha256_hex;
use accelerator::launch::outbound::resolve::{
    FetchVerifyCacheResolver, ResolverConfig, HOST_PLATFORM,
};

const FIXTURE: &str = env!("CARGO_BIN_EXE_accelerator-fixture");
// Matches the launcher's own version (anti-rollback) regardless of version bump.
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BINARY: &str = "frobnicate";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tempdir(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "res-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("mkdir temp");
    dir
}

fn minisign_bin() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("minisign"))
        .find(|candidate| candidate.is_file())
}

/// Generate an unencrypted keypair; return (public-key file contents, secret
/// key path).
fn generate_keypair(
    minisign: &Path,
    dir: &Path,
    name: &str,
) -> (String, PathBuf) {
    let public = dir.join(format!("{name}.pub"));
    let secret = dir.join(format!("{name}.key"));
    let status = Command::new(minisign)
        .args(["-G", "-W", "-f", "-p"])
        .arg(&public)
        .arg("-s")
        .arg(&secret)
        .output()
        .expect("run minisign -G");
    assert!(status.status.success(), "keygen failed");
    (std::fs::read_to_string(&public).expect("read pub"), secret)
}

/// Sign `bytes` with `secret`, returning the `.minisig` contents.
fn sign(minisign: &Path, secret: &Path, dir: &Path, bytes: &[u8]) -> String {
    let target = dir.join(format!(
        "payload-{}",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&target, bytes).expect("write payload");
    let signature = target.with_extension("minisig");
    let status = Command::new(minisign)
        .arg("-S")
        .arg("-s")
        .arg(secret)
        .arg("-x")
        .arg(&signature)
        .arg("-m")
        .arg(&target)
        .output()
        .expect("run minisign -S");
    assert!(status.status.success(), "signing failed");
    std::fs::read_to_string(&signature).expect("read sig")
}

fn manifest_json(version: &str, sha256: &str, signature: &str) -> String {
    // The .minisig contents carry newlines and tabs; JSON-escape both.
    let escaped = signature.replace('\n', "\\n").replace('\t', "\\t");
    format!(
        "{{\"schema_version\":1,\"version\":\"{version}\",\"binaries\":{{\
         \"{BINARY}\":{{\"description\":\"Frobnicator\",\"platforms\":{{\
         \"{HOST_PLATFORM}\":{{\"sha256\":\"{sha256}\",\"signature\":\"{escaped}\"\
         }}}}}}}}}}"
    )
}

struct Harness {
    server: MockServer,
    cache: PathBuf,
    trusted: Vec<String>,
    fixture_bytes: Vec<u8>,
    workdir: PathBuf,
    minisign: PathBuf,
    trusted_secret: PathBuf,
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

    fn resolver(&self) -> FetchVerifyCacheResolver {
        let fetcher =
            Fetcher::with_backoff(std::time::Duration::from_millis(1))
                .expect("fetcher");
        FetchVerifyCacheResolver::with_fetcher(
            self.config(self.server.base_url()),
            self.keys(),
            fetcher,
        )
    }

    fn resolve(&self) -> Result<PathBuf, ResolutionError> {
        self.resolver().resolve(&ExternalCommand {
            name: OsString::from(BINARY),
            args: vec![],
        })
    }
}

fn asset_path() -> String {
    format!("/{BINARY}-{HOST_PLATFORM}")
}

/// Build a harness with a correctly-signed release the resolver will accept.
fn happy_harness() -> Option<Harness> {
    let minisign = minisign_bin()?;
    let workdir = tempdir("work");
    let cache = tempdir("cache");
    let (trusted_pub, trusted_secret) =
        generate_keypair(&minisign, &workdir, "release");
    let fixture_bytes = std::fs::read(FIXTURE).expect("read fixture");
    let sha = sha256_hex(&fixture_bytes);
    let asset_sig = sign(&minisign, &trusted_secret, &workdir, &fixture_bytes);
    let manifest = manifest_json(VERSION, &sha, &asset_sig);
    let manifest_sig =
        sign(&minisign, &trusted_secret, &workdir, manifest.as_bytes());

    let server = MockServer::start();
    server.route("/manifest.json", Route::Ok(manifest.into_bytes()));
    server.route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));
    server.route(&asset_path(), Route::Ok(fixture_bytes.clone()));

    Some(Harness {
        server,
        cache,
        trusted: vec![trusted_pub],
        fixture_bytes,
        workdir,
        minisign,
        trusted_secret,
    })
}

macro_rules! skip_if_no_minisign {
    ($harness:expr) => {
        match $harness {
            Some(harness) => harness,
            None => {
                eprintln!("skipping: minisign not on PATH");
                return Ok(());
            }
        }
    };
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
        harness.server.hits(&asset_path()),
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
    harness
        .server
        .route("/manifest.json", Route::Ok(manifest.into_bytes()));
    harness
        .server
        .route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));

    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::ChecksumMismatch { .. })
    ));
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
    harness
        .server
        .route("/manifest.json", Route::Ok(manifest.into_bytes()));
    harness
        .server
        .route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));

    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::SignatureMismatch { .. })
    ));
    Ok(())
}

#[test]
fn a_tampered_manifest_signature_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // Manifest bytes differ from what manifest.minisig signs.
    let tampered = manifest_json(VERSION, &"a".repeat(64), "junk");
    harness
        .server
        .route("/manifest.json", Route::Ok(tampered.into_bytes()));
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
    harness
        .server
        .route("/manifest.json", Route::Ok(manifest.into_bytes()));
    harness
        .server
        .route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));

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
         \"{BINARY}\":{{\"description\":\"Frobnicator\",\"platforms\":{{\
         \"{HOST_PLATFORM}\":{{\"sha256\":\"{sha}\",\"signature\":\"{escaped}\"\
         }}}}}}}}}}"
    );
    let manifest_sig = sign(
        &harness.minisign,
        &harness.trusted_secret,
        &harness.workdir,
        manifest.as_bytes(),
    );
    harness
        .server
        .route("/manifest.json", Route::Ok(manifest.into_bytes()));
    harness
        .server
        .route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));

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
    harness.server.route("/manifest.json", Route::Status(404));
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::ReleaseUnavailable { .. })
    ));
    Ok(())
}

#[test]
fn a_missing_asset_is_named() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    harness.server.route(&asset_path(), Route::Status(404));
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
    harness.server.route("/manifest.json", Route::Status(500));
    // Exhausted 5xx retries map to Fetch (a 404 would be ReleaseUnavailable).
    assert!(matches!(
        harness.resolve(),
        Err(ResolutionError::Fetch { .. })
    ));
    assert_eq!(harness.server.hits("/manifest.json"), 3, "bounded retries");
    Ok(())
}

#[test]
fn a_transient_5xx_recovers_within_the_retry_budget(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // Fail the asset fetch twice then succeed; its signature is already inline.
    harness.server.route(
        &asset_path(),
        Route::FlakyThenOk {
            fail_times: 2,
            body: harness.fixture_bytes.clone(),
        },
    );
    let path = harness.resolve()?;
    assert!(path.exists());
    assert_eq!(harness.server.hits(&asset_path()), 3);
    Ok(())
}

#[test]
fn a_redirect_to_a_disallowed_host_is_refused() -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    // 127.0.0.1 is off the redirect allowlist; the unfollowed 302 is a transport
    // failure (Fetch), not a 404 (ReleaseUnavailable).
    harness.server.route(
        "/manifest.json",
        Route::Redirect(format!("{}/elsewhere", harness.server.base_url())),
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
    let offline = FetchVerifyCacheResolver::with_fetcher(
        harness.config("http://127.0.0.1:1".to_owned()),
        harness.keys(),
        Fetcher::with_backoff(std::time::Duration::from_millis(1))?,
    );
    let path = offline.resolve(&ExternalCommand {
        name: OsString::from(BINARY),
        args: vec![],
    })?;
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
        harness.server.hits(&asset_path()) >= 2,
        "a refetch must have occurred"
    );
    Ok(())
}

#[test]
fn a_poisoned_cache_entry_offline_is_a_distinct_diagnostic(
) -> Result<(), Box<dyn Error>> {
    let harness = skip_if_no_minisign!(happy_harness());
    let path = harness.resolve()?;
    std::fs::write(&path, b"poisoned")?;
    // No server to re-fetch from → the distinct corrupt-and-refetch diagnostic.
    let offline = FetchVerifyCacheResolver::with_fetcher(
        harness.config("http://127.0.0.1:1".to_owned()),
        harness.keys(),
        Fetcher::with_backoff(std::time::Duration::from_millis(1))?,
    );
    assert!(matches!(
        offline.resolve(&ExternalCommand {
            name: OsString::from(BINARY),
            args: vec![],
        }),
        Err(ResolutionError::CorruptCacheAndRefetchFailed { .. })
    ));
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
