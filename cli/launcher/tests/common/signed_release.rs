//! A release signed with a freshly generated key and served from a loopback
//! mock server, so a resolver trusting that key fetches a real sub-binary.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use http_test_support::{MockHTTPServer, RequestKey, Route};
use sha2::{Digest as _, Sha256};

use accelerator::launch::outbound::resolve::keys::TrustedKeys;
use accelerator::launch::outbound::resolve::verifier::sha256_hex;
use accelerator::launch::outbound::resolve::HOST_PLATFORM;

pub const FIXTURE: &str = env!("CARGO_BIN_EXE_accelerator-fixture");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BINARY: &str = "visualiser";

pub fn minisign_bin() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("minisign"))
        .find(|candidate| candidate.is_file())
}

/// Generate an unencrypted keypair; return (public-key file contents, secret
/// key path).
pub fn generate_keypair(
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
pub fn sign(
    minisign: &Path,
    secret: &Path,
    dir: &Path,
    bytes: &[u8],
) -> String {
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

fn escaped(signature: &str) -> String {
    // The .minisig contents carry newlines and tabs; JSON-escape both.
    signature.replace('\n', "\\n").replace('\t', "\\t")
}

pub fn manifest_json(version: &str, sha256: &str, signature: &str) -> String {
    manifest_with_trees(version, sha256, signature, &[])
}

fn manifest_with_trees(
    version: &str,
    sha256: &str,
    signature: &str,
    trees: &[PublishedTree],
) -> String {
    let signature = escaped(signature);
    let artifacts = trees
        .iter()
        .map(PublishedTree::manifest_entry)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"version\":\"{version}\",\"binaries\":{{\
         \"{BINARY}\":{{\"description\":\"Visualiser\",\"platforms\":{{\
         \"{HOST_PLATFORM}\":{{\"sha256\":\"{sha256}\",\"signature\":\"{signature}\"\
         }}}}}}}},\"artifacts\":{{{artifacts}}}}}"
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// A tree archive of one file naming its artifact, so each artifact's digest
/// differs, with the signed attestation the resolver checks it against.
pub struct PublishedTree {
    pub artifact: String,
    pub digest: String,
    archive: Vec<u8>,
    archive_signature: String,
    attestation: String,
    attestation_signature: String,
}

impl PublishedTree {
    fn build(
        artifact: &str,
        minisign: &Path,
        secret: &Path,
        workdir: &Path,
    ) -> Self {
        let body = artifact.as_bytes();
        let table = format!(
            "version 1\nf\t644\t{}\t{}\tmarker\n",
            body.len(),
            hex(&Sha256::digest(body))
        );
        let mut builder = tar::Builder::new(Vec::new());
        for (path, bytes) in [(".files", table.as_bytes()), ("marker", body)] {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            builder
                .append_data(&mut header, path, bytes)
                .expect("append");
        }
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        );
        encoder
            .write_all(&builder.into_inner().expect("finish tar"))
            .expect("gzip");
        let archive = encoder.finish().expect("finish gzip");
        let digest = hex(&Sha256::digest(&archive));
        let attestation = format!(
            "{{\"attestation_format_version\":1,\"artifact\":\"{artifact}\",\
             \"platform\":\"{HOST_PLATFORM}\",\"archive_sha256\":\"{digest}\",\
             \"uncompressed_size\":{},\"entry_count\":1,\
             \"table_sha256\":\"{}\"}}",
            body.len(),
            hex(&Sha256::digest(table.as_bytes()))
        );
        Self {
            artifact: artifact.to_owned(),
            archive_signature: sign(minisign, secret, workdir, &archive),
            attestation_signature: sign(
                minisign,
                secret,
                workdir,
                attestation.as_bytes(),
            ),
            digest,
            archive,
            attestation,
        }
    }

    fn asset_path(&self) -> String {
        format!("/accelerator-{}-{HOST_PLATFORM}.tar.gz", self.artifact)
    }

    fn manifest_entry(&self) -> String {
        format!(
            "\"{}\":{{\"description\":\"tree\",\"platforms\":{{\
             \"{HOST_PLATFORM}\":{{\"sha256\":\"{}\",\"signature\":\"{}\",\
             \"archive_size\":{},\"uncompressed_size\":4096,\
             \"entry_count\":1}}}}}}",
            self.artifact,
            self.digest,
            escaped(&self.archive_signature),
            self.archive.len()
        )
    }

    fn serve_on(&self, server: &MockHTTPServer) {
        let asset = self.asset_path();
        for (path, body) in [
            (asset.clone(), self.archive.clone()),
            (
                format!("{asset}.sealed"),
                self.attestation.clone().into_bytes(),
            ),
            (
                format!("{asset}.sealed.sig"),
                self.attestation_signature.clone().into_bytes(),
            ),
        ] {
            server.route(
                RequestKey::get(&path),
                Route::Bytes { status: 200, body },
            );
        }
    }
}

pub fn asset_path() -> String {
    format!("/accelerator-{BINARY}-{HOST_PLATFORM}")
}

pub struct SignedRelease {
    pub server: MockHTTPServer,
    pub trusted: Vec<String>,
    pub fixture_bytes: Vec<u8>,
    pub sha: String,
    pub asset_sig: String,
    pub minisign: PathBuf,
    pub trusted_secret: PathBuf,
    pub trees: Vec<PublishedTree>,
}

impl SignedRelease {
    /// `None` when `minisign` is not on `PATH`.
    pub fn serve(workdir: &Path) -> Option<Self> {
        Self::serve_with_trees(workdir, &[])
    }

    /// As [`Self::serve`], also publishing a tree for each of `artifacts` on
    /// this host.
    pub fn serve_with_trees(
        workdir: &Path,
        artifacts: &[&str],
    ) -> Option<Self> {
        let minisign = minisign_bin()?;
        let (trusted_pub, trusted_secret) =
            generate_keypair(&minisign, workdir, "release");
        let fixture_bytes = std::fs::read(FIXTURE).expect("read fixture");
        let sha = sha256_hex(&fixture_bytes);
        let asset_sig =
            sign(&minisign, &trusted_secret, workdir, &fixture_bytes);
        let trees: Vec<PublishedTree> = artifacts
            .iter()
            .map(|artifact| {
                PublishedTree::build(
                    artifact,
                    &minisign,
                    &trusted_secret,
                    workdir,
                )
            })
            .collect();
        let manifest = manifest_with_trees(VERSION, &sha, &asset_sig, &trees);
        let manifest_sig =
            sign(&minisign, &trusted_secret, workdir, manifest.as_bytes());

        let server = MockHTTPServer::start();
        for (path, body) in [
            ("/manifest.json".to_owned(), manifest.into_bytes()),
            ("/manifest.minisig".to_owned(), manifest_sig.into_bytes()),
            (asset_path(), fixture_bytes.clone()),
        ] {
            server.route(
                RequestKey::get(&path),
                Route::Bytes { status: 200, body },
            );
        }
        for tree in &trees {
            tree.serve_on(&server);
        }
        Some(Self {
            server,
            trusted: vec![trusted_pub],
            fixture_bytes,
            sha,
            asset_sig,
            minisign,
            trusted_secret,
            trees,
        })
    }

    /// As [`Self::serve`], except that under CI an absent `minisign` fails
    /// the test: it is pinned in `mise.toml`, so its absence there is a
    /// misconfiguration rather than a reason to skip.
    pub fn serve_or_skip(workdir: &Path) -> Option<Self> {
        Self::serve_or_skip_with_trees(workdir, &[])
    }

    pub fn serve_or_skip_with_trees(
        workdir: &Path,
        artifacts: &[&str],
    ) -> Option<Self> {
        let release = Self::serve_with_trees(workdir, artifacts);
        if release.is_none() {
            assert!(
                std::env::var_os("CI").is_none(),
                "minisign is required under CI and was not on PATH"
            );
            eprintln!("skipping: minisign not on PATH");
        }
        release
    }

    pub fn keys(&self) -> TrustedKeys {
        TrustedKeys::from_public_key_files(
            &self.trusted.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .expect("trusted keys")
    }

    pub fn asset_hits(&self) -> usize {
        self.server.hits(&RequestKey::get(&asset_path()))
    }

    /// Each published tree's digest on this host, as a launcher pins them.
    pub fn tree_digests(&self) -> BTreeMap<(String, String), String> {
        self.trees
            .iter()
            .map(|tree| {
                (
                    (tree.artifact.clone(), HOST_PLATFORM.to_owned()),
                    tree.digest.clone(),
                )
            })
            .collect()
    }
}
