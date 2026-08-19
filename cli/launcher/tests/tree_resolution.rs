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

use common::{MockServer, Route};
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

use accelerator::launch::core::tree::{
    AcquireSealedTree, Clock, MaterialiseTree, VerifyTree,
};
use accelerator::launch::outbound::resolve::cache_root::probe_attempts;
use accelerator::launch::outbound::resolve::fetcher::Fetcher;
use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::tree::lease::{
    probe_liveness, Liveness,
};
use accelerator::launch::outbound::resolve::tree::{
    ExpectedDigests, TreeResolver,
};
use accelerator::launch::outbound::resolve::HOST_PLATFORM;
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARTIFACT: &str = "driver";

/// A clock the tests do not need to advance.
struct StoppedClock;
impl Clock for StoppedClock {
    fn now_seconds(&self) -> u64 {
        0
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

fn manifest_json(
    archive_sha: &str,
    signature: &str,
    archive_len: u64,
) -> String {
    let escaped = signature.replace('\n', "\\n").replace('\t', "\\t");
    format!(
        "{{\"schema_version\":1,\"version\":\"{VERSION}\",\"binaries\":{{}},\
         \"artifacts\":{{\"{ARTIFACT}\":{{\"description\":\"driver\",\
         \"platforms\":{{\"{HOST_PLATFORM}\":{{\"sha256\":\"{archive_sha}\",\
         \"signature\":\"{escaped}\",\"archive_size\":{archive_len},\
         \"uncompressed_size\":999999,\"entry_count\":9}}}}}}}}}}"
    )
}

struct Harness {
    server: MockServer,
    cache: PathBuf,
    trusted: String,
    archive_sha: String,
    _workdir: TempDir,
    _cache: TempDir,
}

impl Harness {
    fn resolver(&self) -> TreeResolver<'_> {
        let mut digests = BTreeMap::new();
        digests.insert(
            (ARTIFACT.to_owned(), HOST_PLATFORM.to_owned()),
            self.archive_sha.clone(),
        );
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
            clock: &StoppedClock,
            launcher_id: "test-install".to_owned(),
            expected_digests: ExpectedDigests::Fixed(digests),
        }
    }

    fn keys(&self) -> TrustedKeys {
        TrustedKeys::from_public_key_files(&[self.trusted.as_str()])
            .expect("trusted keys")
    }

    fn hits(&self, path: &str) -> usize {
        self.server.hits(path)
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

    let manifest =
        manifest_json(&archive_sha, &archive_sig, archive.len() as u64);
    let manifest_sig =
        sign(minisign, &secret, workdir.path(), manifest.as_bytes());

    let server = MockServer::start();
    server.route("/manifest.json", Route::Ok(manifest.into_bytes()));
    server.route("/manifest.minisig", Route::Ok(manifest_sig.into_bytes()));
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

    Harness {
        server,
        cache: cache.path().to_path_buf(),
        trusted: trusted_pub,
        archive_sha,
        _workdir: workdir,
        _cache: cache,
    }
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
        probe_liveness(&acquired.tree.lease_path),
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
