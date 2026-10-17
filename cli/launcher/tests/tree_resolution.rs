//! End-to-end tree resolution against the mock server and a real minisign
//! keypair. Builds a signed archive, attestation and manifest, then drives the
//! `TreeResolver` through materialise, a warm hit, and an acquire.
//!
//! Requires the `minisign` CLI. Under `CI` its absence is a hard failure rather
//! than a skip, so the hit path's only cryptographic anchor can never silently
//! go unexercised.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use common::{MockHTTPServer, Route};
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

use accelerator::launch::core::tree::{
    AcquireSealedTree, Clock, MaterialiseTree, TreeError, VerifyTree,
};
use accelerator::launch::outbound::resolve::cache_root::probe_attempts;
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::tree::lease::{
    FlockProbe, Liveness, LivenessProbe,
};
use accelerator::launch::outbound::resolve::tree::{
    ExpectedDigests, MaterialiseStep, NoSteps, StepObserver, TreeResolver,
};
use accelerator::launch::outbound::resolve::HOST_PLATFORM;
use lease::probe_shared_lease;
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARTIFACT: &str = "driver";
const BYSTANDER: &str = "browser";

/// Distinct from the harness's archive digest, which a keep set holding only
/// `ARTIFACT`'s pin would spare by accident.
fn bystander_digest() -> String {
    "b".repeat(64)
}

fn pinning(artifact: &str, digest: &str) -> BTreeMap<(String, String), String> {
    BTreeMap::from([(
        (artifact.to_owned(), HOST_PLATFORM.to_owned()),
        digest.to_owned(),
    )])
}

/// A clock the tests do not need to advance.
struct StoppedClock;
impl Clock for StoppedClock {
    fn now_seconds(&self) -> u64 {
        0
    }
    fn sleep_poll_interval(&self) {}
}

/// A clock pinned to a chosen second, for exercising the claim window.
struct FixedClock(u64);
impl Clock for FixedClock {
    fn now_seconds(&self) -> u64 {
        self.0
    }
    fn sleep_poll_interval(&self) {}
}

fn minisign_bin() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("minisign"))
        .find(|candidate| candidate.is_file())
}

/// Resolve minisign, failing hard under CI so the anchor is never skipped.
macro_rules! minisign_or_skip {
    () => {
        match minisign_bin() {
            Some(path) => path,
            None if std::env::var_os("CI").is_some() => {
                panic!("minisign is required under CI and was not on PATH")
            }
            None => {
                eprintln!("skipping: minisign not on PATH");
                return;
            }
        }
    };
}

fn generate_keypair(minisign: &Path, dir: &Path) -> (String, PathBuf) {
    let public = dir.join("release.pub");
    let secret = dir.join("release.key");
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

fn sign(minisign: &Path, secret: &Path, dir: &Path, bytes: &[u8]) -> String {
    let payload = tempfile::Builder::new()
        .prefix("payload-")
        .tempfile_in(dir)
        .expect("payload tempfile");
    std::fs::write(payload.path(), bytes).expect("write payload");
    let signature = payload.path().with_extension("minisig");
    let status = Command::new(minisign)
        .arg("-S")
        .arg("-s")
        .arg(secret)
        .arg("-x")
        .arg(&signature)
        .arg("-m")
        .arg(payload.path())
        .output()
        .expect("run minisign -S");
    assert!(status.status.success(), "signing failed");
    std::fs::read_to_string(&signature).expect("read sig")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// One file the synthetic tree contains.
struct Member {
    path: &'static str,
    mode: u32,
    body: &'static [u8],
}

const MEMBERS: &[Member] = &[
    Member {
        path: "lib",
        mode: 0o755,
        body: b"",
    },
    Member {
        path: "lib/data.pak",
        mode: 0o644,
        body: b"resource bytes",
    },
    Member {
        path: "node",
        mode: 0o755,
        body: b"#!/bin/sh\necho v20\n",
    },
];

/// Build the gzipped tar with the `.files` table first, returning the archive
/// bytes and the table's own sha256.
fn build_archive() -> (Vec<u8>, String, u64, u64) {
    let mut table = String::from("version 1\n");
    let mut uncompressed = 0_u64;
    let mut count = 0_u64;
    for member in MEMBERS {
        if member.path == "lib" {
            table.push_str("d\t755\t0\t-\tlib\n");
        } else {
            let digest = hex(&Sha256::digest(member.body));
            let _ = writeln!(
                table,
                "f\t{:o}\t{}\t{digest}\t{}",
                member.mode,
                member.body.len(),
                member.path
            );
            uncompressed += member.body.len() as u64;
        }
        count += 1;
    }
    // A symlink, so the round trip and verify's link-target detection are
    // exercised.
    table.push_str("l\t777\t0\t-\tlib/current\tdata.pak\n");
    count += 1;
    let table_sha = hex(&Sha256::digest(table.as_bytes()));

    let mut builder = tar::Builder::new(Vec::new());
    append(&mut builder, ".files", 0o644, table.as_bytes());
    for member in MEMBERS {
        if member.path == "lib" {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(0o755);
            header.set_size(0);
            builder
                .append_data(&mut header, "lib", std::io::empty())
                .expect("append dir");
        } else {
            append(&mut builder, member.path, member.mode, member.body);
        }
    }
    let mut link_header = tar::Header::new_gnu();
    link_header.set_entry_type(tar::EntryType::Symlink);
    link_header.set_mode(0o777);
    link_header.set_size(0);
    builder
        .append_link(&mut link_header, "lib/current", "data.pak")
        .expect("append symlink");
    let tar = builder.into_inner().expect("finish tar");
    let mut encoder =
        flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&tar).expect("gzip");
    let archive = encoder.finish().expect("finish gzip");
    (archive, table_sha, uncompressed, count)
}

fn append(
    builder: &mut tar::Builder<Vec<u8>>,
    path: &str,
    mode: u32,
    body: &[u8],
) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(mode);
    header.set_size(body.len() as u64);
    builder
        .append_data(&mut header, path, body)
        .expect("append");
}

fn attestation_json(
    archive_sha: &str,
    uncompressed: u64,
    count: u64,
    table_sha: &str,
) -> String {
    format!(
        "{{\"attestation_format_version\":1,\"artifact\":\"{ARTIFACT}\",\
         \"platform\":\"{HOST_PLATFORM}\",\"archive_sha256\":\"{archive_sha}\",\
         \"uncompressed_size\":{uncompressed},\"entry_count\":{count},\
         \"table_sha256\":\"{table_sha}\"}}"
    )
}

/// What the manifest publishes for `ARTIFACT` on the host platform.
struct ManifestEntry {
    signature: String,
    archive_size: u64,
    uncompressed_size: u64,
}

fn manifest_json(archive_sha: &str, entry: &ManifestEntry) -> String {
    let ManifestEntry {
        signature,
        archive_size,
        uncompressed_size,
    } = entry;
    let escaped = signature.replace('\n', "\\n").replace('\t', "\\t");
    format!(
        "{{\"schema_version\":1,\"version\":\"{VERSION}\",\"binaries\":{{}},\
         \"artifacts\":{{\"{ARTIFACT}\":{{\"description\":\"driver\",\
         \"platforms\":{{\"{HOST_PLATFORM}\":{{\"sha256\":\"{archive_sha}\",\
         \"signature\":\"{escaped}\",\"archive_size\":{archive_size},\
         \"uncompressed_size\":{uncompressed_size},\"entry_count\":9}}}}}}}}}}"
    )
}

struct Harness {
    server: MockHTTPServer,
    cache: PathBuf,
    trusted: String,
    archive_sha: String,
    archive_bytes: Vec<u8>,
    archive_signature: String,
    minisign: PathBuf,
    secret: PathBuf,
    workdir: TempDir,
    _cache: TempDir,
}

impl Harness {
    fn resolver(&self) -> TreeResolver<'_> {
        self.resolver_with(&StoppedClock, &FlockProbe, &NoSteps)
    }

    fn resolver_with<'a>(
        &'a self,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        steps: &'a dyn StepObserver,
    ) -> TreeResolver<'a> {
        self.resolver_over(
            &self.archive_sha,
            "test-install",
            clock,
            liveness,
            steps,
        )
    }

    /// A second launcher sharing this cache and release, expecting `digest`.
    fn resolver_expecting<'a>(
        &'a self,
        digest: &str,
        clock: &'a dyn Clock,
    ) -> TreeResolver<'a> {
        self.resolver_over(
            digest,
            "second-install",
            clock,
            &FlockProbe,
            &NoSteps,
        )
    }

    fn resolver_also_pinning_bystander(&self) -> TreeResolver<'_> {
        let mut digests = pinning(ARTIFACT, &self.archive_sha);
        digests.extend(pinning(BYSTANDER, &bystander_digest()));
        self.resolver_pinning(
            digests,
            "test-install",
            &StoppedClock,
            &FlockProbe,
            &NoSteps,
        )
    }

    fn resolver_over<'a>(
        &'a self,
        digest: &str,
        launcher_id: &str,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        steps: &'a dyn StepObserver,
    ) -> TreeResolver<'a> {
        self.resolver_pinning(
            pinning(ARTIFACT, digest),
            launcher_id,
            clock,
            liveness,
            steps,
        )
    }

    fn resolver_pinning<'a>(
        &'a self,
        digests: BTreeMap<(String, String), String>,
        launcher_id: &str,
        clock: &'a dyn Clock,
        liveness: &'a dyn LivenessProbe,
        steps: &'a dyn StepObserver,
    ) -> TreeResolver<'a> {
        TreeResolver {
            cache_root: self.cache.clone(),
            base_url: self.server.base_url(),
            platform: HOST_PLATFORM.to_owned(),
            expected_version: VERSION.to_owned(),
            keys: Box::leak(Box::new(self.keys())),
            fetcher: Box::leak(Box::new(
                Fetcher::with_backoff(std::time::Duration::from_millis(1))
                    .expect("fetcher"),
            )),
            clock,
            liveness,
            launcher_id: launcher_id.to_owned(),
            expected_digests: ExpectedDigests::Fixed(digests),
            waiter_bound: std::time::Duration::from_secs(5),
            steps,
        }
    }

    fn keys(&self) -> TrustedKeys {
        TrustedKeys::from_public_key_files(&[self.trusted.as_str()])
            .expect("trusted keys")
    }

    fn hits(&self, path: &str) -> usize {
        self.server.hits(path)
    }

    fn sign(&self, bytes: &[u8]) -> String {
        sign(&self.minisign, &self.secret, self.workdir.path(), bytes)
    }

    fn default_entry(&self) -> ManifestEntry {
        ManifestEntry {
            signature: self.archive_signature.clone(),
            archive_size: self.archive_bytes.len() as u64,
            uncompressed_size: 999_999,
        }
    }

    fn serve_manifest(&self, entry: &ManifestEntry) {
        let manifest = manifest_json(&self.archive_sha, entry);
        let signature = self.sign(manifest.as_bytes());
        self.server
            .route("/manifest.json", Route::Ok(manifest.into_bytes()));
        self.server
            .route("/manifest.minisig", Route::Ok(signature.into_bytes()));
    }
}

fn happy_harness(minisign: &Path) -> Harness {
    let workdir = tempfile::tempdir().expect("workdir");
    let cache = tempfile::tempdir().expect("cache");
    let (trusted_pub, secret) = generate_keypair(minisign, workdir.path());

    let (archive, table_sha, uncompressed, count) = build_archive();
    let archive_sha = hex(&Sha256::digest(&archive));
    let archive_sig = sign(minisign, &secret, workdir.path(), &archive);

    let attestation =
        attestation_json(&archive_sha, uncompressed, count, &table_sha);
    let attestation_sig =
        sign(minisign, &secret, workdir.path(), attestation.as_bytes());

    let archive_bytes = archive.clone();
    let server = MockHTTPServer::start();
    let asset = format!("/accelerator-{ARTIFACT}-{HOST_PLATFORM}.tar.gz");
    server.route(&asset, Route::Ok(archive));
    server.route(
        &format!("{asset}.sealed"),
        Route::Ok(attestation.into_bytes()),
    );
    server.route(
        &format!("{asset}.sealed.sig"),
        Route::Ok(attestation_sig.into_bytes()),
    );

    let harness = Harness {
        server,
        cache: cache.path().to_path_buf(),
        trusted: trusted_pub,
        archive_sha,
        archive_bytes,
        archive_signature: archive_sig,
        minisign: minisign.to_owned(),
        secret,
        workdir,
        _cache: cache,
    };
    harness.serve_manifest(&harness.default_entry());
    harness
}

fn archive_path() -> String {
    format!("/accelerator-{ARTIFACT}-{HOST_PLATFORM}.tar.gz")
}

#[test]
fn materialise_fetches_verifies_seals_and_a_warm_hit_touches_no_network() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    // A cold materialise fetches, verifies and seals.
    let sealed = resolver
        .materialise(ARTIFACT)
        .expect("cold materialise succeeds");
    assert_eq!(sealed.artifact, ARTIFACT);
    assert!(sealed.path.join("lib/data.pak").exists());
    assert_eq!(
        std::fs::read(sealed.path.join("lib/data.pak")).unwrap(),
        b"resource bytes"
    );
    // The node binary kept its executable bit through the seal.
    let node_mode = std::fs::metadata(sealed.path.join("node"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(node_mode, 0o555);

    let fetched_once = harness.hits(&archive_path());
    assert_eq!(
        fetched_once, 1,
        "the archive should be fetched exactly once"
    );

    // A warm query is a hit with zero further HTTP.
    let hit = resolver.query(ARTIFACT).expect("query").expect("a hit");
    assert_eq!(hit.digest, harness.archive_sha);
    assert_eq!(
        harness.hits(&archive_path()),
        fetched_once,
        "a warm hit must issue no archive fetch"
    );
    assert_eq!(
        harness.hits("/manifest.json"),
        1,
        "a warm hit must not reload the manifest"
    );
}

#[test]
fn acquire_holds_a_lease_and_a_cold_materialise_probes_the_root_once() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    // A cold materialise probes the cache root exactly once; acquire never does.
    let before = probe_attempts();
    resolver.materialise(ARTIFACT).expect("materialise");
    assert_eq!(
        probe_attempts() - before,
        1,
        "materialise probes the cache root exactly once"
    );

    let before = probe_attempts();
    let acquired = resolver.acquire(ARTIFACT).expect("acquire").expect("a hit");
    assert_eq!(
        probe_attempts() - before,
        0,
        "acquire must not probe the cache root"
    );
    // The lease is held: a prune-style exclusive probe would see it as live.
    assert_eq!(
        probe_shared_lease(&acquired.tree.lease_path),
        Liveness::Held,
        "acquire must hold the tree's lease"
    );
    drop(acquired);
}

#[test]
fn a_second_resolution_of_the_same_digest_reuses_the_generation() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    let first = resolver.materialise(ARTIFACT).expect("first");
    let fetches = harness.hits(&archive_path());

    // A second materialise finds the existing generation via the reuse scan and
    // fetches nothing more.
    let second = resolver.materialise(ARTIFACT).expect("second");
    assert_eq!(first.path, second.path, "the generation should be reused");
    assert_eq!(
        harness.hits(&archive_path()),
        fetches,
        "a reuse must issue no further archive fetch"
    );
}

/// The design executor leases a cold-path tree only at its `.lease` sibling,
/// so a layout change here would silently leave every such tree unleased.
#[test]
fn every_sealed_tree_carries_its_lease_beside_it() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    let fresh = resolver.materialise(ARTIFACT).expect("fresh");
    let reused = resolver.materialise(ARTIFACT).expect("reused");

    for sealed in [fresh, reused] {
        assert_eq!(sealed.lease_path, lease::lease_beside(&sealed.path));
    }
}

#[test]
fn verify_reports_a_sound_tree_and_detects_a_substitution() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    let sealed = resolver.materialise(ARTIFACT).expect("materialise");
    let report = resolver.verify(ARTIFACT).expect("verify");
    assert!(report.is_sound(), "a fresh tree verifies clean: {report:?}");

    // Substitute a same-size, same-mode file — exactly what a stat-only check
    // would miss and the table digest exists to catch.
    let victim = sealed.path.join("lib/data.pak");
    let mut perms = std::fs::metadata(&victim).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&victim, perms).unwrap();
    std::fs::write(&victim, b"substituted!!!").unwrap();

    let report = resolver.verify(ARTIFACT).expect("verify");
    assert!(
        !report.is_sound(),
        "a same-size substitution must be detected"
    );
}

/// A crash observer that errors after a chosen publish step.
struct CrashAfter(MaterialiseStep);
impl StepObserver for CrashAfter {
    fn after(&self, step: MaterialiseStep) -> Result<(), TreeError> {
        if step == self.0 {
            Err(TreeError::Extraction {
                detail: format!("injected crash after {step:?}"),
            })
        } else {
            Ok(())
        }
    }
}

/// A clock the test advances by hand, so the bounded waiter times out because
/// time moved rather than because the test slept.
struct AdvancingClock(std::sync::atomic::AtomicU64);
impl Clock for AdvancingClock {
    fn now_seconds(&self) -> u64 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn sleep_poll_interval(&self) {
        // Each poll advances a virtual second rather than sleeping.
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn a_crash_at_each_publish_step_leaves_only_reclaimable_garbage() {
    let minisign = minisign_or_skip!();
    let steps = [
        MaterialiseStep::ArchiveStreamed,
        MaterialiseStep::ArchiveVerified,
        MaterialiseStep::Extracted,
        MaterialiseStep::Sealed,
        MaterialiseStep::SidecarsWritten,
        MaterialiseStep::LeaseHeld,
        MaterialiseStep::Renamed,
    ];
    for step in steps {
        // A fresh cache per step so residues do not accumulate across cases.
        let harness = happy_harness(&minisign);

        let crash = CrashAfter(step);
        let outcome = harness
            .resolver_with(&StoppedClock, &FlockProbe, &crash)
            .materialise(ARTIFACT);
        assert!(
            outcome.is_err(),
            "the injected crash after {step:?} must abort materialise"
        );

        // No pointer was published, so a query is a miss.
        assert!(
            harness.resolver().query(ARTIFACT).expect("query").is_none(),
            "a crash after {step:?} left a resolvable pointer"
        );

        // The reaper, run with a clock well past the age backstop, removes the
        // residue and leaves the trees directory holding no generation.
        let far_future =
            AdvancingClock(std::sync::atomic::AtomicU64::new(1 << 40));
        harness
            .resolver_with(&far_future, &FlockProbe, &NoSteps)
            .prune(ARTIFACT, None)
            .expect("prune");
        let residue = residual_generations(&harness);
        assert!(
            residue.is_empty(),
            "a crash after {step:?} left unreclaimed residue: {residue:?}"
        );
    }
}

#[test]
fn a_failed_winner_releases_the_lock_and_the_next_makes_progress() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    // The winner crashes mid-materialisation.
    let crash = CrashAfter(MaterialiseStep::Extracted);
    assert!(harness
        .resolver_with(&StoppedClock, &FlockProbe, &crash)
        .materialise(ARTIFACT)
        .is_err());

    // The next materialise takes the same lock — proving it was released — and
    // succeeds.
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the next materialise makes progress");
    assert!(sealed.path.join("lib/data.pak").exists());
}

#[test]
fn the_bounded_waiter_expires_with_materialisation_in_progress() {
    use accelerator::launch::outbound::resolve::tree::lease::take_single_flight;

    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    // Hold the single-flight lock so every try fails, standing in for a winner
    // still downloading.
    let lock_path = harness
        .cache
        .join("trees")
        .join(format!(".tmp-{ARTIFACT}-{HOST_PLATFORM}.lock"));
    std::fs::create_dir_all(harness.cache.join("trees")).unwrap();
    let _held = take_single_flight(&lock_path).expect("hold the lock");

    // The waiter advances its own clock each poll, so it crosses the bound
    // without any real sleep, and yields the non-sticky in-progress cause.
    let clock = AdvancingClock(std::sync::atomic::AtomicU64::new(0));
    let outcome = harness
        .resolver_with(&clock, &FlockProbe, &NoSteps)
        .materialise(ARTIFACT);
    assert!(
        matches!(outcome, Err(TreeError::MaterialisationInProgress { .. })),
        "a held lock past the bound must yield materialisation-in-progress, \
         got {outcome:?}"
    );
}

#[test]
fn two_concurrent_cold_resolutions_issue_exactly_one_archive_fetch() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            harness.resolver().materialise(ARTIFACT)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            harness.resolver().materialise(ARTIFACT)
        });
        (a.join().unwrap(), b.join().unwrap())
    });

    let first = results.0.expect("first materialise");
    let second = results.1.expect("second materialise");
    assert_eq!(
        first.path, second.path,
        "both resolutions must land on one generation"
    );
    assert_eq!(
        harness.hits(&archive_path()),
        1,
        "two concurrent cold resolutions must fetch the archive exactly once"
    );
}

/// A generation directory currently under trees/ (ignoring sidecars, temps and
/// the claims dir).
fn residual_generations(harness: &Harness) -> Vec<String> {
    let trees = harness.cache.join("trees");
    let Ok(entries) = std::fs::read_dir(&trees) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_generation = entry.path().is_dir()
                && !name.starts_with(".tmp-")
                && name != "claims";
            is_generation.then_some(name)
        })
        .collect()
}

use accelerator::launch::cache;
use accelerator::launch::inbound::cli::CacheAction;

fn run_cache(
    harness: &Harness,
    action: &CacheAction,
) -> Result<Vec<u8>, kernel::Error> {
    let mut out = Vec::new();
    cache::run(action, &harness.resolver(), &mut out)?;
    Ok(out)
}

#[test]
fn cache_verify_reports_ok_on_a_clean_tree_and_fails_on_a_corrupt_one() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    let out = run_cache(
        &harness,
        &CacheAction::Verify {
            name: Some(ARTIFACT.to_owned()),
        },
    )
    .expect("verify clean");
    assert!(String::from_utf8_lossy(&out).contains("ok"));

    // Corrupt a file with a same-size substitution.
    let victim = sealed.path.join("lib/data.pak");
    let mut perms = std::fs::metadata(&victim).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&victim, perms).unwrap();
    std::fs::write(&victim, b"substituted!!!").unwrap();

    let outcome = run_cache(
        &harness,
        &CacheAction::Verify {
            name: Some(ARTIFACT.to_owned()),
        },
    );
    assert!(
        matches!(outcome, Err(kernel::Error::Failed(_))),
        "a corrupt tree must make verify fail"
    );
}

#[test]
fn cache_repair_restores_a_corrupt_tree() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    // Truncate a file — a verify failure repair must heal.
    let victim = sealed.path.join("lib/data.pak");
    let mut perms = std::fs::metadata(&victim).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&victim, perms).unwrap();
    std::fs::write(&victim, b"").unwrap();

    run_cache(
        &harness,
        &CacheAction::Repair {
            name: Some(ARTIFACT.to_owned()),
            force: false,
        },
    )
    .expect("repair");

    // The tree verifies clean again.
    run_cache(
        &harness,
        &CacheAction::Verify {
            name: Some(ARTIFACT.to_owned()),
        },
    )
    .expect("verify after repair");
}

#[test]
fn cache_repair_force_rematerialises_a_clean_tree() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let fetches = harness.hits(&archive_path());

    // A forced repair of a clean tree fetches again rather than no-opping.
    run_cache(
        &harness,
        &CacheAction::Repair {
            name: Some(ARTIFACT.to_owned()),
            force: true,
        },
    )
    .expect("forced repair");
    assert!(
        harness.hits(&archive_path()) > fetches,
        "repair --force must re-materialise even a clean tree"
    );
}

#[test]
fn cache_ensure_prints_the_tree_and_lease_paths() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    let out = run_cache(
        &harness,
        &CacheAction::Ensure {
            names: vec![ARTIFACT.to_owned()],
        },
    )
    .expect("ensure");
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains(ARTIFACT), "ensure names the artifact");
    assert!(text.contains(".lease"), "ensure prints the lease path");
}

#[test]
fn cache_ensure_emits_a_cause_envelope_when_the_archive_is_unreachable() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness.server.route(&archive_path(), Route::Status(404));

    let outcome = run_cache(
        &harness,
        &CacheAction::Ensure {
            names: vec![ARTIFACT.to_owned()],
        },
    );
    assert!(
        matches!(&outcome, Err(kernel::Error::Failed(_))),
        "expected a structured ensure failure, got {outcome:?}"
    );
    let envelope = outcome
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        envelope.contains(r#""error":"ensure-failed""#),
        "{envelope}"
    );
    assert!(envelope.contains(r#""cause":"unreachable""#), "{envelope}");
    assert!(envelope.contains(ARTIFACT), "{envelope}");
}

#[test]
fn every_cache_verb_refuses_an_unknown_name_without_touching_the_filesystem() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    for action in [
        CacheAction::Verify {
            name: Some("nonesuch".to_owned()),
        },
        CacheAction::Repair {
            name: Some("nonesuch".to_owned()),
            force: false,
        },
        CacheAction::Ensure {
            names: vec!["nonesuch".to_owned()],
        },
    ] {
        let outcome = run_cache(&harness, &action);
        assert!(
            matches!(outcome, Err(kernel::Error::Refusal(_))),
            "an unknown name must be refused: {outcome:?}"
        );
    }
    // Nothing was fetched for the unknown name.
    assert_eq!(harness.hits(&archive_path()), 0);
}

#[test]
fn cache_prune_reclaims_owned_orphans_and_keeps_unowned_ones() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let paths = tree_paths(&harness);
    std::fs::create_dir_all(paths.root()).unwrap();
    let owned =
        paths.temp_generation(&sibling_generation(&harness.archive_sha));
    std::fs::create_dir(&owned).unwrap();
    let unowned = paths.root().join(".tmp-orphan");
    std::fs::create_dir(&unowned).unwrap();

    let out = run_cache(&harness, &CacheAction::Prune { older_than: None })
        .expect("prune");

    assert!(
        String::from_utf8_lossy(&out).contains("reclaimed 1 entries"),
        "{}",
        String::from_utf8_lossy(&out)
    );
    assert!(!owned.exists(), "prune left an owned orphan temp tree");
    assert!(unowned.exists(), "prune removed residue nobody owns");
}

fn lock_files(harness: &Harness) -> Vec<String> {
    let mut locks: Vec<String> = std::fs::read_dir(tree_paths(harness).root())
        .unwrap()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name.starts_with(".tmp-")
                && std::path::Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension == "lock")
        })
        .collect();
    locks.sort();
    locks
}

/// Run a resolution against `harness` under the 10s bound, asserting that it
/// neither creates nor removes a lock file.
fn within_bound<T: Send + 'static>(
    harness: &Arc<Harness>,
    run: impl FnOnce(&Harness) -> T + Send + 'static,
) -> T {
    let locks_before = lock_files(harness);
    let bounded = Arc::clone(harness);
    let outcome = common::within(
        std::time::Duration::from_secs(10),
        "the resolution",
        move || run(&bounded),
    );
    assert_eq!(lock_files(harness), locks_before, "a lock file changed");
    outcome
}

/// A materialise creates its own lock file, so the lock-file set can only be
/// compared across one once both artifacts' lock files exist.
fn create_free_tree_locks(harness: &Harness) {
    use accelerator::launch::outbound::resolve::tree::lease::take_single_flight;

    let paths = tree_paths(harness);
    std::fs::create_dir_all(paths.root()).unwrap();
    for artifact in [ARTIFACT, BYSTANDER] {
        drop(
            take_single_flight(
                &paths.single_flight_lock(artifact, HOST_PLATFORM),
            )
            .unwrap(),
        );
    }
}

fn plant_bystanders_pinned_archive(harness: &Harness) -> PathBuf {
    let archive = tree_paths(harness).temp_archive(
        BYSTANDER,
        HOST_PLATFORM,
        &bystander_digest(),
    );
    std::fs::write(&archive, b"resumable").unwrap();
    archive
}

#[test]
fn another_artifacts_pinned_archive_survives_a_materialise() {
    let minisign = minisign_or_skip!();
    let harness = Arc::new(happy_harness(&minisign));
    create_free_tree_locks(&harness);
    let pinned = plant_bystanders_pinned_archive(&harness);

    within_bound(&harness, |harness| {
        harness
            .resolver_also_pinning_bystander()
            .materialise(ARTIFACT)
    })
    .expect("materialise");

    assert!(pinned.exists(), "the bystander's pinned archive was reaped");
}

#[test]
fn another_artifacts_pinned_archive_survives_a_prune() {
    let minisign = minisign_or_skip!();
    let harness = Arc::new(happy_harness(&minisign));
    create_free_tree_locks(&harness);
    let pinned = plant_bystanders_pinned_archive(&harness);

    within_bound(&harness, |harness| {
        harness
            .resolver_also_pinning_bystander()
            .prune(ARTIFACT, None)
    })
    .expect("prune");

    assert!(pinned.exists(), "the bystander's pinned archive was reaped");
}

use accelerator::launch::outbound::resolve::tree::claims;

/// Publish a second pointer for a *different* digest (a sibling install's pin)
/// by materialising, then relabelling the generation, pointer and attestation
/// under a fabricated digest. Returns that digest.
fn plant_sibling_generation(harness: &Harness) -> String {
    // A distinct digest a sibling install would carry.
    let sibling_digest = "d".repeat(64);
    let trees = harness.cache.join("trees");
    // A minimal generation directory and pointer for the sibling digest; the
    // reclamation reads pointers and claims, not the tree's contents.
    let generation = format!(
        "{ARTIFACT}-{HOST_PLATFORM}-{sibling_digest}-1-0011223344556677"
    );
    std::fs::create_dir_all(trees.join(&generation)).unwrap();
    std::fs::write(
        trees.join(format!("{ARTIFACT}-{HOST_PLATFORM}-{sibling_digest}.ref")),
        &generation,
    )
    .unwrap();
    sibling_digest
}

#[test]
fn prune_spares_a_sibling_claim_inside_the_window_and_reclaims_a_stale_one() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    // The running launcher's own tree, freshly claimed.
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    let sibling = plant_sibling_generation(&harness);
    let paths =
        accelerator::launch::outbound::resolve::tree::layout::TreePaths::under(
            &harness.cache,
        );

    // A sibling claim refreshed just now — a fortnight-aware clock spares it.
    let present = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    claims::refresh(&paths, &sibling, "sibling-install", &FixedClock(present));

    let sibling_pointer = harness
        .cache
        .join("trees")
        .join(format!("{ARTIFACT}-{HOST_PLATFORM}-{sibling}.ref"));

    // With a present-time clock the sibling's fresh claim spares its generation.
    harness
        .resolver_with(&FixedClock(present), &FlockProbe, &NoSteps)
        .prune(ARTIFACT, None)
        .expect("prune");
    assert!(
        sibling_pointer.exists(),
        "a sibling used just now must be spared"
    );

    // A clock a fortnight on sees the same claim as stale, and reclaims it.
    let later = present + claims::CLAIM_WINDOW.as_secs() + 1;
    harness
        .resolver_with(&FixedClock(later), &FlockProbe, &NoSteps)
        .prune(ARTIFACT, None)
        .expect("prune");
    assert!(
        !sibling_pointer.exists(),
        "a sibling whose claim went stale must be reclaimed"
    );
}

#[test]
fn prune_older_than_overrides_the_window() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    let sibling = plant_sibling_generation(&harness);
    let paths =
        accelerator::launch::outbound::resolve::tree::layout::TreePaths::under(
            &harness.cache,
        );
    let present = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    claims::refresh(&paths, &sibling, "sibling-install", &FixedClock(present));

    let sibling_pointer = harness
        .cache
        .join("trees")
        .join(format!("{ARTIFACT}-{HOST_PLATFORM}-{sibling}.ref"));

    // A one-second retention window makes even a just-now claim stale.
    harness
        .resolver_with(&FixedClock(present + 2), &FlockProbe, &NoSteps)
        .prune(ARTIFACT, Some(std::time::Duration::from_secs(1)))
        .expect("prune");
    assert!(
        !sibling_pointer.exists(),
        "--older-than must override the default window"
    );
}

#[test]
fn a_cache_root_at_0775_still_resolves_since_the_launcher_owns_trees() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    // A RHEL/Fedora umask-002 shape: the cache root itself is group-writable.
    std::fs::set_permissions(
        &harness.cache,
        std::fs::Permissions::from_mode(0o775),
    )
    .unwrap();
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("a 0775 cache root must still resolve");
}

#[test]
fn a_symlinked_trees_directory_is_refused_with_a_remediation() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    // Point trees/ at another directory via a symlink — chmod cannot make this
    // safe, so materialise must refuse.
    let elsewhere = harness.cache.join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, harness.cache.join("trees"))
        .unwrap();

    let outcome = harness.resolver().materialise(ARTIFACT);
    assert!(
        matches!(&outcome, Err(kernel_err) if kernel_err.to_string().contains("chmod")),
        "a symlinked trees/ must be refused with a remediation: {outcome:?}"
    );
}

#[test]
fn verify_detects_deletion_mode_change_and_a_changed_symlink() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    // Unseal the tree so the test can mutate it, then apply three distinct
    // corruptions.
    unseal(&sealed.path);
    std::fs::remove_file(sealed.path.join("lib/data.pak")).unwrap();
    std::fs::set_permissions(
        sealed.path.join("node"),
        std::fs::Permissions::from_mode(0o444),
    )
    .unwrap();
    std::fs::remove_file(sealed.path.join("lib/current")).unwrap();
    std::os::unix::fs::symlink("node", sealed.path.join("lib/current"))
        .unwrap();

    let report = harness.resolver().verify(ARTIFACT).expect("verify");
    assert!(!report.is_sound());
    let kinds: Vec<&str> = report
        .findings
        .iter()
        .map(|(_, d)| describe_kind(d))
        .collect();
    assert!(
        kinds.contains(&"missing"),
        "deletion not detected: {kinds:?}"
    );
    assert!(
        kinds.contains(&"mode"),
        "mode change not detected: {kinds:?}"
    );
    assert!(
        kinds.contains(&"link-target"),
        "changed symlink target not detected: {kinds:?}"
    );
}

#[test]
fn verify_detects_an_unexpected_extra_entry() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    unseal(&sealed.path);
    std::fs::write(sealed.path.join("lib/interloper"), b"x").unwrap();

    let report = harness.resolver().verify(ARTIFACT).expect("verify");
    assert!(report
        .findings
        .iter()
        .any(|(path, _)| path == "lib/interloper"));
}

#[test]
fn prune_spares_a_generation_whose_lease_is_still_held() {
    use lease::hold_shared_lease;

    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    // Plant a stale-claimed sibling generation, but hold its lease.
    let sibling = plant_sibling_generation(&harness);
    let paths =
        accelerator::launch::outbound::resolve::tree::layout::TreePaths::under(
            &harness.cache,
        );
    let generation =
        format!("{ARTIFACT}-{HOST_PLATFORM}-{sibling}-1-0011223344556677");
    let _lease = hold_shared_lease(&paths.lease(&generation)).expect("lease");

    // No claim at all, so the sibling is stale — but the held lease spares it.
    harness
        .resolver_with(&StoppedClock, &FlockProbe, &NoSteps)
        .prune(ARTIFACT, Some(std::time::Duration::from_secs(0)))
        .expect("prune");
    assert!(
        paths.generation(&generation).exists(),
        "a leased generation must be spared by prune"
    );
    let _ = sealed;
}

use accelerator::launch::outbound::resolve::tree::layout::TreePaths;

/// Answers every probe the same way, so `Held` and `Unknown` can be exercised
/// without a live holder or a filesystem that lacks `flock`.
struct FixedLiveness(Liveness);

impl LivenessProbe for FixedLiveness {
    fn probe(&self, _lease: &std::path::Path) -> Liveness {
        self.0
    }
}

const BACKSTOP_SECONDS: u64 = 3600;
const ZERO: std::time::Duration = std::time::Duration::from_secs(0);

fn wall_clock_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn aged() -> FixedClock {
    FixedClock(wall_clock_now() + 2 * BACKSTOP_SECONDS)
}

fn tree_paths(harness: &Harness) -> TreePaths {
    TreePaths::under(&harness.cache)
}

fn sibling_generation(digest: &str) -> String {
    format!("{ARTIFACT}-{HOST_PLATFORM}-{digest}-1-0011223344556677")
}

fn plant_unreferenced_generation(harness: &Harness) -> std::path::PathBuf {
    let generation =
        tree_paths(harness).generation(&sibling_generation(&"c".repeat(64)));
    std::fs::create_dir_all(&generation).unwrap();
    generation
}

fn sidecars_of(
    harness: &Harness,
    generation: &std::path::Path,
) -> [std::path::PathBuf; 3] {
    let paths = tree_paths(harness);
    let name = generation_name_of(generation);
    [
        paths.lease(&name),
        paths.attestation(&name),
        paths.attestation_signature(&name),
    ]
}

fn plant_sidecars(harness: &Harness, generation: &std::path::Path) {
    for sidecar in sidecars_of(harness, generation) {
        std::fs::write(sidecar, b"").unwrap();
    }
}

fn assert_sidecars_survive(harness: &Harness, generation: &std::path::Path) {
    for sidecar in sidecars_of(harness, generation) {
        assert!(sidecar.exists(), "{} was removed", sidecar.display());
    }
}

#[test]
fn an_injected_held_probe_spares_a_stale_sibling_in_prune() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let sibling = plant_sibling_generation(&harness);

    harness
        .resolver_with(&StoppedClock, &FixedLiveness(Liveness::Held), &NoSteps)
        .prune(ARTIFACT, Some(ZERO))
        .expect("prune");

    assert!(tree_paths(&harness)
        .generation(&sibling_generation(&sibling))
        .exists());
}

#[test]
fn an_injected_held_probe_spares_a_superseded_generation_in_repair() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    harness
        .resolver_with(&StoppedClock, &FixedLiveness(Liveness::Held), &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");

    assert!(old.path.exists());
}

#[test]
fn an_injected_held_probe_spares_an_aged_orphan_in_prune() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let orphan = plant_unreferenced_generation(&harness);
    plant_sidecars(&harness, &orphan);

    harness
        .resolver_with(&aged(), &FixedLiveness(Liveness::Held), &NoSteps)
        .prune(ARTIFACT, None)
        .expect("prune");

    assert!(orphan.exists());
    assert_sidecars_survive(&harness, &orphan);
}

#[test]
fn an_injected_held_probe_spares_an_aged_orphan_while_materialising() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    std::fs::create_dir_all(tree_paths(&harness).root()).unwrap();
    let orphan = plant_unreferenced_generation(&harness);

    harness
        .resolver_with(&aged(), &FixedLiveness(Liveness::Held), &NoSteps)
        .materialise(ARTIFACT)
        .expect("materialise");

    assert!(orphan.exists());
}

fn fresh() -> FixedClock {
    FixedClock(wall_clock_now())
}

fn sibling_pointer(harness: &Harness, digest: &str) -> std::path::PathBuf {
    harness
        .cache
        .join("trees")
        .join(format!("{ARTIFACT}-{HOST_PLATFORM}-{digest}.ref"))
}

/// Prunes an unclaimed sibling at a zero window and reports whether its
/// generation and pointer both survived; panics if only one did.
fn unclaimed_sibling_kept(
    liveness: Liveness,
    clock: fn() -> FixedClock,
) -> bool {
    let minisign = minisign_bin().expect("minisign");
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let sibling = plant_sibling_generation(&harness);

    harness
        .resolver_with(&clock(), &FixedLiveness(liveness), &NoSteps)
        .prune(ARTIFACT, Some(ZERO))
        .expect("prune");

    let generation = tree_paths(&harness)
        .generation(&sibling_generation(&sibling))
        .exists();
    let pointer = sibling_pointer(&harness, &sibling).exists();
    assert_eq!(generation, pointer, "the generation and its pointer split");
    generation
}

#[test]
fn an_unclaimed_held_generation_is_kept_once_aged() {
    let _ = minisign_or_skip!();
    assert!(unclaimed_sibling_kept(Liveness::Held, aged));
}

#[test]
fn an_unclaimed_free_generation_is_reclaimed_while_young() {
    let _ = minisign_or_skip!();
    assert!(!unclaimed_sibling_kept(Liveness::Free, fresh));
}

#[test]
fn an_unclaimed_free_generation_is_reclaimed_once_aged() {
    let _ = minisign_or_skip!();
    assert!(!unclaimed_sibling_kept(Liveness::Free, aged));
}

#[test]
fn an_unclaimed_unknown_generation_is_kept_while_young() {
    let _ = minisign_or_skip!();
    assert!(unclaimed_sibling_kept(Liveness::Unknown, fresh));
}

#[test]
fn an_unclaimed_unknown_generation_is_reclaimed_once_aged() {
    let _ = minisign_or_skip!();
    assert!(!unclaimed_sibling_kept(Liveness::Unknown, aged));
}

fn generation_name_of(sealed: &std::path::Path) -> String {
    sealed.file_name().unwrap().to_str().unwrap().to_owned()
}

fn back_date_two_days(path: &std::path::Path) {
    let then = std::time::SystemTime::now()
        - std::time::Duration::from_secs(2 * 86_400);
    std::fs::File::open(path)
        .unwrap()
        .set_modified(then)
        .unwrap();
}

fn mtime_of(path: &std::path::Path) -> std::time::SystemTime {
    std::fs::symlink_metadata(path).unwrap().modified().unwrap()
}

#[test]
fn a_superseded_free_generation_is_reclaimed() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    harness
        .resolver_with(&fresh(), &FixedLiveness(Liveness::Free), &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");

    assert!(!old.path.exists());
}

#[test]
fn a_superseded_unknown_generation_is_kept() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    harness
        .resolver_with(&fresh(), &FixedLiveness(Liveness::Unknown), &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");

    assert!(old.path.exists());
    assert_sidecars_survive(&harness, &old.path);
}

#[test]
fn a_kept_unknown_superseded_generation_is_reaped_once_aged() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let unknown = FixedLiveness(Liveness::Unknown);

    harness
        .resolver_with(&fresh(), &unknown, &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");
    assert!(old.path.exists());

    harness
        .resolver_with(&aged(), &unknown, &NoSteps)
        .prune(ARTIFACT, None)
        .expect("prune");

    assert!(!old.path.exists());
    let paths = tree_paths(&harness);
    let generation = generation_name_of(&old.path);
    for sidecar in [
        paths.attestation(&generation),
        paths.attestation_signature(&generation),
        paths.lease(&generation),
    ] {
        assert!(!sidecar.exists(), "{} survived", sidecar.display());
    }
}

#[test]
fn an_old_unknown_superseded_generation_is_kept_for_a_full_backstop() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    back_date_two_days(&old.path);
    let unknown = FixedLiveness(Liveness::Unknown);

    harness
        .resolver_with(&fresh(), &unknown, &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");
    assert!(old.path.exists());

    let almost_a_backstop_later =
        FixedClock(wall_clock_now() + BACKSTOP_SECONDS - 60);
    harness
        .resolver_with(&almost_a_backstop_later, &unknown, &NoSteps)
        .prune(ARTIFACT, None)
        .expect("prune");

    assert!(old.path.exists());
}

#[test]
fn a_held_superseded_generation_keeps_its_extraction_mtime() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let old = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    back_date_two_days(&old.path);
    let extracted = mtime_of(&old.path);

    harness
        .resolver_with(&fresh(), &FixedLiveness(Liveness::Held), &NoSteps)
        .repair(ARTIFACT, true)
        .expect("forced repair");

    assert_eq!(mtime_of(&old.path), extracted);
}

/// Make a sealed tree writable so a test can corrupt it.
fn unseal(root: &std::path::Path) {
    fn walk(dir: &std::path::Path) {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))
            .ok();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ft = entry.file_type().unwrap();
                if ft.is_dir() {
                    walk(&path);
                } else if !ft.is_symlink() {
                    std::fs::set_permissions(
                        &path,
                        std::fs::Permissions::from_mode(0o644),
                    )
                    .ok();
                }
            }
        }
    }
    walk(root);
}

const fn describe_kind(
    d: &accelerator::launch::core::tree::Discrepancy,
) -> &'static str {
    use accelerator::launch::core::tree::Discrepancy;
    match d {
        Discrepancy::Missing => "missing",
        Discrepancy::Unexpected => "unexpected",
        Discrepancy::Size { .. } => "size",
        Discrepancy::Mode { .. } => "mode",
        Discrepancy::Digest => "digest",
        Discrepancy::LinkTarget { .. } => "link-target",
    }
}

#[test]
fn an_interrupted_download_resumes_with_a_range_request() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    // Simulate an interrupted first run: write the first half of the archive to
    // the digest-named temp path the resolver resumes from.
    let archive = harness.archive_bytes.clone();
    let trees = harness.cache.join("trees");
    std::fs::create_dir_all(&trees).unwrap();
    let temp = trees.join(format!(
        ".tmp-{ARTIFACT}-{HOST_PLATFORM}-{}.archive",
        harness.archive_sha
    ));
    std::fs::write(&temp, &archive[..archive.len() / 2]).unwrap();

    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("resume completes");
    assert!(sealed.path.join("lib/data.pak").exists());

    let asset = archive_path();
    assert_eq!(
        harness.server.range_hits(&asset),
        1,
        "resume must issue exactly one Range request"
    );
    // The resumed tree verifies exactly as a fresh one does.
    harness.resolver().verify(ARTIFACT).expect("verify resumed");
}

fn plant_partial(harness: &Harness, bytes: &[u8]) -> PathBuf {
    let paths = tree_paths(harness);
    std::fs::create_dir_all(paths.root()).unwrap();
    let partial =
        paths.temp_archive(ARTIFACT, HOST_PLATFORM, &harness.archive_sha);
    std::fs::write(&partial, bytes).unwrap();
    partial
}

fn with_trailing_bytes(archive: &[u8], extra: usize) -> Vec<u8> {
    let mut longer = archive.to_vec();
    longer.extend(std::iter::repeat_n(0_u8, extra));
    longer
}

fn serve_a_loose_size_bound(harness: &Harness) {
    harness.serve_manifest(&ManifestEntry {
        archive_size: harness.archive_bytes.len() as u64 + 1024,
        ..harness.default_entry()
    });
}

#[test]
fn a_complete_partial_is_verified_without_refetching() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    plant_partial(&harness, &harness.archive_bytes);

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("a complete partial materialises");

    assert_eq!(harness.hits(&archive_path()), 0);
}

#[test]
fn a_complete_partial_under_a_loose_size_bound_is_confirmed_by_the_server() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    serve_a_loose_size_bound(&harness);
    plant_partial(&harness, &harness.archive_bytes);

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("a complete partial materialises");

    assert_eq!(harness.hits(&archive_path()), 1);
    assert_eq!(harness.server.range_hits(&archive_path()), 1);
}

#[test]
fn a_partial_longer_than_the_served_file_under_a_loose_bound_is_refetched_whole(
) {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    serve_a_loose_size_bound(&harness);
    plant_partial(&harness, &with_trailing_bytes(&harness.archive_bytes, 10));

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the refetched archive materialises");

    assert_eq!(harness.server.range_hits(&archive_path()), 1);
    assert_eq!(harness.hits(&archive_path()), 2);
}

#[test]
fn an_over_long_partial_is_refetched_whole() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    plant_partial(&harness, &with_trailing_bytes(&harness.archive_bytes, 1));

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the refetched archive materialises");

    assert_eq!(harness.server.range_hits(&archive_path()), 0);
    assert_eq!(harness.hits(&archive_path()), 1);
}

#[test]
fn a_resumed_download_that_fails_mid_body_retries_from_the_agreed_offset() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let archive = harness.archive_bytes.clone();
    harness
        .server
        .route(&archive_path(), Route::FailFirstRange(archive.clone()));
    plant_partial(&harness, &archive[..archive.len() / 2]);

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the retried resume materialises");
}

fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .unwrap();
}

#[test]
fn an_unreadable_partial_is_discarded_so_the_next_run_recovers() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let partial = plant_partial(&harness, &harness.archive_bytes);
    set_mode(&partial, 0o000);
    if std::fs::File::open(&partial).is_ok() {
        eprintln!("skipping: this process reads files it has no access to");
        return;
    }

    let outcome = harness.resolver().materialise(ARTIFACT);

    assert!(
        matches!(&outcome, Err(TreeError::Extraction { detail })
            if detail.contains("cannot re-read")),
        "got {outcome:?}"
    );
    assert!(!partial.exists(), "the unreadable partial survived");
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the next run recovers");
}

#[test]
fn an_unwritable_short_partial_is_replaced_in_the_same_run() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let archive = harness.archive_bytes.clone();
    let partial = plant_partial(&harness, &archive[..archive.len() / 2]);
    set_mode(&partial, 0o444);
    if std::fs::OpenOptions::new()
        .write(true)
        .open(&partial)
        .is_ok()
    {
        eprintln!("skipping: this process writes files it has no access to");
        return;
    }

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the unwritable partial is replaced");
}

fn with_a_byte_flipped(bytes: &[u8]) -> Vec<u8> {
    let mut flipped = bytes.to_vec();
    flipped[bytes.len() / 2] ^= 0xff;
    flipped
}

fn assert_a_corrupt_partial_is_discarded(harness: &Harness, planted: &[u8]) {
    let partial = plant_partial(harness, planted);

    let outcome = harness.resolver().materialise(ARTIFACT);

    assert!(
        matches!(outcome, Err(TreeError::UnexpectedDigest { .. })),
        "got {outcome:?}"
    );
    assert!(!partial.exists(), "the corrupt partial survived");
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the next run recovers");
}

#[test]
fn a_corrupt_partial_archive_is_discarded_so_the_next_run_recovers() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let archive = harness.archive_bytes.clone();

    assert_a_corrupt_partial_is_discarded(
        &harness,
        &with_a_byte_flipped(&archive[..archive.len() / 2]),
    );
}

#[test]
fn a_complete_corrupt_partial_is_discarded_so_the_next_run_recovers() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let archive = harness.archive_bytes.clone();

    assert_a_corrupt_partial_is_discarded(
        &harness,
        &with_a_byte_flipped(&archive),
    );
}

#[test]
fn a_partial_whose_signature_fails_is_kept_for_the_next_run() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let partial = plant_partial(&harness, &harness.archive_bytes);
    harness.serve_manifest(&ManifestEntry {
        signature: harness.sign(b"other bytes"),
        ..harness.default_entry()
    });

    let outcome = harness.resolver().materialise(ARTIFACT);

    assert!(
        matches!(&outcome, Err(TreeError::Attestation { detail })
            if detail.contains("does not verify under")),
        "got {outcome:?}"
    );
    assert!(partial.exists(), "a partial matching its pin was discarded");
    harness.serve_manifest(&harness.default_entry());
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("the next run verifies the kept partial");
    assert_eq!(harness.hits(&archive_path()), 0);
}

#[test]
fn the_free_space_check_counts_the_partial_already_on_disk() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let uncompressed_size = 1_u64 << 50;
    harness.serve_manifest(&ManifestEntry {
        uncompressed_size,
        ..harness.default_entry()
    });
    let archive = harness.archive_bytes.clone();
    plant_partial(&harness, &archive[..archive.len() / 2]);

    let outcome = harness.resolver().materialise(ARTIFACT);

    let length = archive.len() as u64;
    assert!(
        matches!(outcome, Err(TreeError::DiskShortfall { needed, .. })
            if needed == uncompressed_size + length - length / 2),
        "got {outcome:?}"
    );
}

#[test]
fn a_hit_still_resolves_on_a_read_only_cache_root() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");

    // Make trees/ and its contents read-only, as a warm populated cache may be.
    // The claim refresh is best-effort, so a hit must still succeed.
    let claims = harness.cache.join("trees/claims");
    make_read_only(&claims);

    let hit = harness.resolver().query(ARTIFACT).expect("query");
    assert!(
        hit.is_some(),
        "a read-only cache root must still resolve a hit"
    );

    // Restore writability so the tempdir cleans up.
    let _ = std::fs::set_permissions(
        &claims,
        std::fs::Permissions::from_mode(0o755),
    );
}

/// Recursively make a directory tree read-only.
fn make_read_only(dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                make_read_only(&entry.path());
            }
        }
    }
    let _ =
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555));
}

#[test]
fn a_corrupt_archive_is_refused_before_anything_is_extracted() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();

    // Serve bytes whose digest cannot match the signed one, so verification
    // (step 5) rejects before extraction (step 6) creates a temp generation.
    let mut corrupt = harness.archive_bytes.clone();
    let midpoint = corrupt.len() / 2;
    corrupt[midpoint] ^= 0xff;
    harness.server.route(&archive_path(), Route::Ok(corrupt));

    let outcome = resolver.materialise(ARTIFACT);
    assert!(
        matches!(outcome, Err(TreeError::UnexpectedDigest { .. })),
        "a corrupt archive must be a digest refusal, got {outcome:?}"
    );
    assert!(
        !tree_paths(&harness)
            .temp_archive(ARTIFACT, HOST_PLATFORM, &harness.archive_sha)
            .exists(),
        "the corrupt download was kept"
    );

    // Nothing was extracted: no generation directory and no temp generation
    // directory (both are the only directories materialise creates) exist.
    let trees = harness.cache.join("trees");
    let created_a_directory = std::fs::read_dir(&trees)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            entry.path().is_dir() && name != "claims"
        });
    assert!(
        !created_a_directory,
        "a corrupt archive was extracted before it was verified"
    );
    assert!(
        resolver.query(ARTIFACT).expect("query").is_none(),
        "a rejected archive must leave no resolvable pointer"
    );
}

#[test]
fn two_launchers_with_the_same_digest_share_one_generation_offline() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let manifest_loads = harness.hits("/manifest.json");
    let archive_fetches = harness.hits(&archive_path());

    // A different build — a different expected version, pointed at an
    // unreachable host — whose compiled-in digest is the same shares the tree.
    let mut digests = BTreeMap::new();
    digests.insert(
        (ARTIFACT.to_owned(), HOST_PLATFORM.to_owned()),
        harness.archive_sha.clone(),
    );
    let keys = harness.keys();
    let fetcher = Fetcher::with_backoff(std::time::Duration::from_millis(1))
        .expect("fetcher");
    let launcher_b = TreeResolver {
        cache_root: harness.cache.clone(),
        base_url: "http://127.0.0.1:1".to_owned(),
        platform: HOST_PLATFORM.to_owned(),
        expected_version: "99.99.99".to_owned(),
        keys: &keys,
        fetcher: &fetcher,
        clock: &StoppedClock,
        launcher_id: "launcher-b".to_owned(),
        expected_digests: ExpectedDigests::Fixed(digests),
        waiter_bound: std::time::Duration::from_secs(5),
        liveness: &FlockProbe,
        steps: &NoSteps,
    };

    let hit = launcher_b.query(ARTIFACT).expect("query").expect("a hit");
    assert_eq!(hit.digest, harness.archive_sha);
    assert_eq!(
        harness.hits("/manifest.json"),
        manifest_loads,
        "a cross-version hit must not reload the manifest"
    );
    assert_eq!(
        harness.hits(&archive_path()),
        archive_fetches,
        "a cross-version hit must issue no archive fetch"
    );
}

#[test]
fn a_manifest_naming_a_different_digest_is_a_refusal_not_a_fetch() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);

    // This launcher expects a digest the correctly-signed manifest does not
    // name, so materialise must refuse rather than fetch what the manifest lists.
    let expected = "0".repeat(64);
    let paths = tree_paths(&harness);
    std::fs::create_dir_all(paths.root()).unwrap();
    let partial = paths.temp_archive(ARTIFACT, HOST_PLATFORM, &expected);
    std::fs::write(&partial, b"resumable").unwrap();
    let mut digests = BTreeMap::new();
    digests.insert((ARTIFACT.to_owned(), HOST_PLATFORM.to_owned()), expected);
    let keys = harness.keys();
    let fetcher = Fetcher::with_backoff(std::time::Duration::from_millis(1))
        .expect("fetcher");
    let resolver = TreeResolver {
        cache_root: harness.cache.clone(),
        base_url: harness.server.base_url(),
        platform: HOST_PLATFORM.to_owned(),
        expected_version: VERSION.to_owned(),
        keys: &keys,
        fetcher: &fetcher,
        clock: &StoppedClock,
        launcher_id: "test-install".to_owned(),
        expected_digests: ExpectedDigests::Fixed(digests),
        waiter_bound: std::time::Duration::from_secs(5),
        liveness: &FlockProbe,
        steps: &NoSteps,
    };

    let outcome = resolver.materialise(ARTIFACT);
    assert!(
        matches!(outcome, Err(TreeError::UnexpectedDigest { .. })),
        "a manifest digest disagreement must be a refusal, got {outcome:?}"
    );
    assert_eq!(
        harness.hits(&archive_path()),
        0,
        "a digest disagreement must not fetch the archive"
    );
    assert!(partial.exists(), "a refused manifest discarded a partial");
}

#[test]
fn an_acquire_succeeds_on_a_populated_cache_with_the_host_unreachable() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    resolver.materialise(ARTIFACT).expect("materialise");

    // Every release request now fails; the hit path touches no network, so a
    // populated cache still resolves.
    harness.server.route("/manifest.json", Route::Status(503));
    harness
        .server
        .route("/manifest.minisig", Route::Status(503));
    harness.server.route(&archive_path(), Route::Status(503));

    assert!(
        resolver.acquire(ARTIFACT).expect("acquire").is_some(),
        "an unreachable host must not defeat a populated cache"
    );
}

#[test]
fn verify_succeeds_with_the_host_unreachable() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    resolver.materialise(ARTIFACT).expect("materialise");

    harness.server.route("/manifest.json", Route::Status(503));
    harness.server.route(&archive_path(), Route::Status(503));

    let report = resolver.verify(ARTIFACT).expect("verify");
    assert!(
        report.is_sound(),
        "verify is local and must not need the release host: {report:?}"
    );
}

#[test]
fn a_table_rewritten_to_match_a_substitution_is_caught_by_its_signed_digest() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    let sealed = resolver.materialise(ARTIFACT).expect("materialise");

    // The archive is discarded after materialisation, so the `.files` table's
    // only remaining anchor is the signed `table_sha256`. Substitute a file and
    // rewrite the table row to match it: the per-row walk would pass, but the
    // table's own digest no longer matches the attestation.
    unseal(&sealed.path);
    let substitute = b"substituted!!!";
    assert_eq!(substitute.len(), b"resource bytes".len());
    std::fs::write(sealed.path.join("lib/data.pak"), substitute).unwrap();

    let table_path = sealed.path.join(".files");
    let table = std::fs::read_to_string(&table_path).unwrap();
    let new_digest = hex(&Sha256::digest(substitute));
    let rewritten = table
        .lines()
        .map(|line| {
            if line.ends_with("\tlib/data.pak") {
                let mut fields: Vec<&str> = line.split('\t').collect();
                fields[3] = new_digest.as_str();
                fields.join("\t")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&table_path, format!("{rewritten}\n")).unwrap();

    let outcome = resolver.verify(ARTIFACT);
    assert!(
        matches!(&outcome, Err(TreeError::Extraction { detail })
            if detail.contains("signed digest")),
        "a table rewritten to match a substitution must be caught by its \
         signed digest, got {outcome:?}"
    );
}

#[test]
fn a_symlinked_generation_or_group_writable_pointer_is_a_miss() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    let sealed = resolver.materialise(ARTIFACT).expect("materialise");
    assert!(resolver.query(ARTIFACT).expect("query").is_some());

    // A generation directory replaced by a symlink — even one pointing at an
    // otherwise-compliant user-owned directory — is refused, not resolved.
    let aside = sealed.path.with_file_name("aside-generation");
    std::fs::rename(&sealed.path, &aside).unwrap();
    std::os::unix::fs::symlink(&aside, &sealed.path).unwrap();
    assert!(
        resolver.query(ARTIFACT).expect("query").is_none(),
        "a symlinked generation must be a miss"
    );

    // Restore the real directory; the hit returns.
    std::fs::remove_file(&sealed.path).unwrap();
    std::fs::rename(&aside, &sealed.path).unwrap();
    assert!(resolver.query(ARTIFACT).expect("query").is_some());

    // A group-writable pointer is refused before its contents become a path.
    let paths =
        accelerator::launch::outbound::resolve::tree::layout::TreePaths::under(
            &harness.cache,
        );
    let pointer = paths.pointer(ARTIFACT, HOST_PLATFORM, &harness.archive_sha);
    std::fs::set_permissions(&pointer, std::fs::Permissions::from_mode(0o660))
        .unwrap();
    assert!(
        resolver.query(ARTIFACT).expect("query").is_none(),
        "a group-writable pointer must be a miss"
    );
}

#[test]
fn a_repair_whose_refetch_fails_leaves_the_previous_tree_resolvable() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    let sealed = resolver.materialise(ARTIFACT).expect("materialise");

    // Corrupt the tree so repair must refetch, then make the refetch fail.
    unseal(&sealed.path);
    std::fs::write(sealed.path.join("lib/data.pak"), b"").unwrap();
    harness.server.route(&archive_path(), Route::Status(404));

    let outcome = resolver.repair(ARTIFACT, false);
    assert!(
        outcome.is_err(),
        "a failed refetch must surface as an error"
    );

    let hit = resolver
        .query(ARTIFACT)
        .expect("query")
        .expect("still a hit");
    assert_eq!(
        hit.path, sealed.path,
        "a failed repair must not disturb the old tree"
    );
}

#[test]
fn a_repair_spares_a_superseded_generation_whose_lease_is_held() {
    use lease::hold_shared_lease;

    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    let old = resolver.materialise(ARTIFACT).expect("materialise");
    let old_generation =
        old.path.file_name().unwrap().to_str().unwrap().to_owned();

    let paths =
        accelerator::launch::outbound::resolve::tree::layout::TreePaths::under(
            &harness.cache,
        );
    // A live consumer holds the old generation's lease across the repair.
    let _held =
        hold_shared_lease(&paths.lease(&old_generation)).expect("lease");

    let fresh = resolver.repair(ARTIFACT, true).expect("forced repair");
    assert_ne!(fresh.path, old.path, "repair must build a new generation");
    assert!(
        old.path.exists(),
        "a leased superseded generation must not be unlinked"
    );
    assert_eq!(
        std::fs::read(old.path.join("lib/data.pak")).unwrap(),
        b"resource bytes",
        "the old generation's files must stay readable to the live consumer"
    );
}

#[test]
fn prune_never_reclaims_the_current_generation_even_at_a_zero_window() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let resolver = harness.resolver();
    let sealed = resolver.materialise(ARTIFACT).expect("materialise");

    // A zero retention window would reclaim any generation not otherwise spared;
    // the one the running launcher is publishing — carrying its own expected
    // digest — must survive, which is what protects the rename/pointer window.
    resolver
        .prune(ARTIFACT, Some(std::time::Duration::from_secs(0)))
        .expect("prune");

    let hit = resolver
        .query(ARTIFACT)
        .expect("query")
        .expect("still a hit");
    assert_eq!(hit.path, sealed.path);
    assert!(sealed.path.exists(), "the current generation was reclaimed");
}

#[test]
fn two_platforms_sharing_one_cache_root_each_resolve_their_own_tree() {
    let minisign = minisign_or_skip!();
    let workdir = tempfile::tempdir().expect("workdir");
    let cache = tempfile::tempdir().expect("cache");
    let (trusted_pub, secret) = generate_keypair(&minisign, workdir.path());

    let (archive, table_sha, uncompressed, count) = build_archive();
    let archive_sha = hex(&Sha256::digest(&archive));
    let archive_sig = sign(&minisign, &secret, workdir.path(), &archive);

    let other_platform = "linux-fake-x64";
    let platforms = [HOST_PLATFORM, other_platform];

    // One signed manifest naming the driver on both platforms.
    let escaped = archive_sig.replace('\n', "\\n").replace('\t', "\\t");
    let entries: Vec<String> = platforms
        .iter()
        .map(|platform| {
            format!(
                "\"{platform}\":{{\"sha256\":\"{archive_sha}\",\
                 \"signature\":\"{escaped}\",\"archive_size\":{},\
                 \"uncompressed_size\":999999,\"entry_count\":9}}",
                archive.len()
            )
        })
        .collect();
    let manifest = format!(
        "{{\"schema_version\":1,\"version\":\"{VERSION}\",\"binaries\":{{}},\
         \"artifacts\":{{\"{ARTIFACT}\":{{\"description\":\"driver\",\
         \"platforms\":{{{}}}}}}}}}",
        entries.join(",")
    );
    let manifest_sig =
        sign(&minisign, &secret, workdir.path(), manifest.as_bytes());

    let server = MockHTTPServer::start();
    server.route("/manifest.json", Route::Ok(manifest.into_bytes()));
    server.route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));
    for platform in platforms {
        let asset = format!("/accelerator-{ARTIFACT}-{platform}.tar.gz");
        server.route(&asset, Route::Ok(archive.clone()));
        let attestation = format!(
            "{{\"attestation_format_version\":1,\"artifact\":\"{ARTIFACT}\",\
             \"platform\":\"{platform}\",\"archive_sha256\":\"{archive_sha}\",\
             \"uncompressed_size\":{uncompressed},\"entry_count\":{count},\
             \"table_sha256\":\"{table_sha}\"}}"
        );
        let attestation_sig =
            sign(&minisign, &secret, workdir.path(), attestation.as_bytes());
        server.route(
            &format!("{asset}.sealed"),
            Route::Ok(attestation.into_bytes()),
        );
        server.route(
            &format!("{asset}.sealed.sig"),
            Route::Ok(attestation_sig.into_bytes()),
        );
    }

    let keys = TrustedKeys::from_public_key_files(&[trusted_pub.as_str()])
        .expect("keys");
    let resolve = |platform: &str| {
        let mut digests = BTreeMap::new();
        digests.insert(
            (ARTIFACT.to_owned(), platform.to_owned()),
            archive_sha.clone(),
        );
        let fetcher =
            Fetcher::with_backoff(std::time::Duration::from_millis(1))
                .expect("fetcher");
        let resolver = TreeResolver {
            cache_root: cache.path().to_path_buf(),
            base_url: server.base_url(),
            platform: platform.to_owned(),
            expected_version: VERSION.to_owned(),
            keys: &keys,
            fetcher: &fetcher,
            clock: &StoppedClock,
            launcher_id: format!("install-{platform}"),
            expected_digests: ExpectedDigests::Fixed(digests),
            waiter_bound: std::time::Duration::from_secs(5),
            liveness: &FlockProbe,
            steps: &NoSteps,
        };
        resolver.materialise(ARTIFACT).expect("materialise").path
    };

    let host_tree = resolve(HOST_PLATFORM);
    let other_tree = resolve(other_platform);
    assert_ne!(
        host_tree, other_tree,
        "each platform gets its own generation"
    );
    assert!(host_tree.exists() && other_tree.exists());
    assert!(host_tree
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains(HOST_PLATFORM));
    assert!(other_tree
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains(other_platform));
}

const LEASE_HOLDER: &str = env!("CARGO_BIN_EXE_launcher-fixture-lease-holder");

/// A transient holder can briefly pin a lease: a `minisign` child a parallel
/// test spawned inside `materialise`'s own lease window inherits it.
const RELEASE_BOUND: std::time::Duration = std::time::Duration::from_secs(60);

/// A detached process holding leases, standing in for a live design daemon. It
/// takes its own leases, so the test process never holds a long-lived lease
/// that a child another test spawns could inherit.
struct LeaseHolder {
    child: std::process::Child,
    terminated: bool,
}

fn spawn_lease_holder(leases: &[PathBuf]) -> LeaseHolder {
    use std::io::BufRead as _;

    let mut child = std::process::Command::new(LEASE_HOLDER)
        .args(leases)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the lease holder");
    let stdout = child.stdout.take().expect("the holder's stdout");
    let mut line = String::new();
    std::io::BufReader::new(stdout)
        .read_line(&mut line)
        .expect("read the holder's readiness");
    let holder = LeaseHolder {
        child,
        terminated: false,
    };
    assert_eq!(line.trim(), "ready", "the lease holder never became ready");
    holder
}

impl LeaseHolder {
    fn terminate(&mut self) {
        if let Some(pid) = i32::try_from(self.child.id())
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        {
            let _ = rustix::process::kill_process_group(
                pid,
                rustix::process::Signal::TERM,
            );
        }
        let _ = self.child.wait();
        self.terminated = true;
    }
}

impl Drop for LeaseHolder {
    fn drop(&mut self) {
        if !self.terminated {
            self.terminate();
        }
    }
}

fn await_released(lease: &Path) {
    let deadline = std::time::Instant::now() + RELEASE_BOUND;
    while probe_shared_lease(lease) == Liveness::Held {
        assert!(
            std::time::Instant::now() < deadline,
            "{} was never released",
            lease.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn own_pointer(harness: &Harness) -> PathBuf {
    sibling_pointer(harness, &harness.archive_sha)
}

#[test]
fn a_second_launcher_spares_generations_a_live_daemon_holds() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let second_digest = "e".repeat(64);
    let second = harness.resolver_expecting(&second_digest, &StoppedClock);

    let mut daemon =
        spawn_lease_holder(std::slice::from_ref(&sealed.lease_path));
    assert_eq!(probe_shared_lease(&sealed.lease_path), Liveness::Held);
    second.prune(ARTIFACT, Some(ZERO)).expect("prune");

    assert!(sealed.path.exists());
    assert!(own_pointer(&harness).exists());
    daemon.terminate();
}

#[test]
fn a_second_launcher_reclaims_generations_no_daemon_holds() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let second_digest = "e".repeat(64);
    let second = harness.resolver_expecting(&second_digest, &StoppedClock);

    await_released(&sealed.lease_path);
    second.prune(ARTIFACT, Some(ZERO)).expect("prune");

    assert!(!sealed.path.exists());
    assert!(!own_pointer(&harness).exists());
}

#[test]
fn a_second_launcher_reclaims_once_the_daemon_group_has_exited() {
    let minisign = minisign_or_skip!();
    let harness = happy_harness(&minisign);
    let sealed = harness
        .resolver()
        .materialise(ARTIFACT)
        .expect("materialise");
    let second_digest = "e".repeat(64);
    let second = harness.resolver_expecting(&second_digest, &StoppedClock);

    let mut daemon =
        spawn_lease_holder(std::slice::from_ref(&sealed.lease_path));
    assert_eq!(probe_shared_lease(&sealed.lease_path), Liveness::Held);
    second.prune(ARTIFACT, Some(ZERO)).expect("prune");
    assert!(sealed.path.exists());

    daemon.terminate();
    await_released(&sealed.lease_path);
    second.prune(ARTIFACT, Some(ZERO)).expect("prune");

    assert!(!sealed.path.exists());
    assert!(!own_pointer(&harness).exists());
}
