//! Black-box tests of the compiled `accelerator-verify` shim against the real
//! minisign CLI. Skips cleanly when `minisign` is not on PATH.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const SHIM: &str = env!("CARGO_BIN_EXE_accelerator-verify");
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn minisign_bin() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join("minisign"))
        .find(|candidate| candidate.is_file())
}

fn tempdir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "verify-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn keypair(minisign: &Path, dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    let public = dir.join(format!("{name}.pub"));
    let secret = dir.join(format!("{name}.key"));
    let out = Command::new(minisign)
        .args(["-G", "-W", "-f", "-p"])
        .arg(&public)
        .arg("-s")
        .arg(&secret)
        .output()
        .expect("minisign -G");
    assert!(out.status.success());
    (public, secret)
}

fn sign(minisign: &Path, secret: &Path, target: &Path) -> PathBuf {
    let signature = target.with_extension("minisig");
    let out = Command::new(minisign)
        .arg("-S")
        .arg("-s")
        .arg(secret)
        .arg("-x")
        .arg(&signature)
        .arg("-m")
        .arg(target)
        .output()
        .expect("minisign -S");
    assert!(out.status.success());
    signature
}

macro_rules! require_minisign {
    () => {
        match minisign_bin() {
            Some(path) => path,
            None => {
                eprintln!("skipping: minisign not on PATH");
                return Ok(());
            }
        }
    };
}

#[test]
fn exits_zero_on_a_valid_signature() -> Result<(), Box<dyn Error>> {
    let minisign = require_minisign!();
    let dir = tempdir();
    let (public, secret) = keypair(&minisign, &dir, "release");
    let target = dir.join("launcher");
    std::fs::write(&target, b"launcher bytes")?;
    let signature = sign(&minisign, &secret, &target);

    let status = Command::new(SHIM)
        .arg(&public)
        .arg(&signature)
        .arg(&target)
        .status()?;
    assert!(status.success());
    Ok(())
}

#[test]
fn exits_nonzero_on_a_tampered_payload() -> Result<(), Box<dyn Error>> {
    let minisign = require_minisign!();
    let dir = tempdir();
    let (public, secret) = keypair(&minisign, &dir, "release");
    let target = dir.join("launcher");
    std::fs::write(&target, b"launcher bytes")?;
    let signature = sign(&minisign, &secret, &target);
    std::fs::write(&target, b"tampered bytes")?; // change after signing

    let status = Command::new(SHIM)
        .arg(&public)
        .arg(&signature)
        .arg(&target)
        .status()?;
    assert!(!status.success(), "tampered payload must be refused");
    Ok(())
}

#[test]
fn exits_nonzero_on_a_non_release_key() -> Result<(), Box<dyn Error>> {
    let minisign = require_minisign!();
    let dir = tempdir();
    let (release_public, _release_secret) = keypair(&minisign, &dir, "release");
    let (_attacker_public, attacker_secret) =
        keypair(&minisign, &dir, "attacker");
    let target = dir.join("launcher");
    std::fs::write(&target, b"launcher bytes")?;
    let signature = sign(&minisign, &attacker_secret, &target);

    let status = Command::new(SHIM)
        .arg(&release_public)
        .arg(&signature)
        .arg(&target)
        .status()?;
    assert!(
        !status.success(),
        "non-release-key signature must be refused"
    );
    Ok(())
}
