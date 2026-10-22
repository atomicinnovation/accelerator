//! A release signed with a freshly generated key and served from a loopback
//! mock server, so a resolver trusting that key fetches a real sub-binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use http_test_support::{MockHTTPServer, RequestKey, Route};

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
pub fn sign(minisign: &Path, secret: &Path, dir: &Path, bytes: &[u8]) -> String {
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

pub fn manifest_json(version: &str, sha256: &str, signature: &str) -> String {
    // The .minisig contents carry newlines and tabs; JSON-escape both.
    let escaped = signature.replace('\n', "\\n").replace('\t', "\\t");
    format!(
        "{{\"schema_version\":1,\"version\":\"{version}\",\"binaries\":{{\
         \"{BINARY}\":{{\"description\":\"Visualiser\",\"platforms\":{{\
         \"{HOST_PLATFORM}\":{{\"sha256\":\"{sha256}\",\"signature\":\"{escaped}\"\
         }}}}}}}}}}"
    )
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
}

impl SignedRelease {
    /// `None` when `minisign` is not on `PATH`.
    pub fn serve(workdir: &Path) -> Option<Self> {
        let minisign = minisign_bin()?;
        let (trusted_pub, trusted_secret) =
            generate_keypair(&minisign, workdir, "release");
        let fixture_bytes = std::fs::read(FIXTURE).expect("read fixture");
        let sha = sha256_hex(&fixture_bytes);
        let asset_sig =
            sign(&minisign, &trusted_secret, workdir, &fixture_bytes);
        let manifest = manifest_json(VERSION, &sha, &asset_sig);
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
        Some(Self {
            server,
            trusted: vec![trusted_pub],
            fixture_bytes,
            sha,
            asset_sig,
            minisign,
            trusted_secret,
        })
    }

    /// As [`Self::serve`], except that under CI an absent `minisign` fails
    /// the test: it is pinned in `mise.toml`, so its absence there is a
    /// misconfiguration rather than a reason to skip.
    pub fn serve_or_skip(workdir: &Path) -> Option<Self> {
        let release = Self::serve(workdir);
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
}
