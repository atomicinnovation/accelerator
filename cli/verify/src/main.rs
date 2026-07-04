//! `accelerator-verify` — the bootstrap's root-of-trust minisign verifier.
//!
//! A binary cannot verify itself, so the bash bootstrap uses this tiny vendored
//! shim to verify the fetched launcher's signature against the plugin-committed
//! public key BEFORE exec, failing closed. It is delivered over the trusted
//! marketplace/plugin channel (its trust basis), pure `minisign-verify` so it
//! links statically with no heavy closure, and held to the same lints as the
//! rest of the workspace — so it propagates `Result` and never `unwrap`s.
//!
//! Usage: `accelerator-verify <public-key-file> <signature-file> <target-file>`
//! Exit 0 iff the signature is valid for the target under the key.

use std::ffi::OsString;
use std::process::ExitCode;

use minisign_verify::{PublicKey, Signature};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("accelerator-verify: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[OsString]) -> Result<(), String> {
    let [public_key_file, signature_file, target_file] = args else {
        return Err("usage: accelerator-verify <pubkey> <signature> <target>"
            .to_owned());
    };

    let public_key_text = read(public_key_file)?;
    let public_key = parse_public_key(&public_key_text)?;

    let signature_text = read(signature_file)?;
    let signature = Signature::decode(&signature_text)
        .map_err(|error| format!("invalid signature: {error}"))?;

    let target = std::fs::read(target_file).map_err(|error| {
        format!("cannot read {}: {error}", display(target_file))
    })?;

    public_key
        .verify(&target, &signature, false)
        .map_err(|error| format!("signature verification failed: {error}"))
}

fn parse_public_key(text: &str) -> Result<PublicKey, String> {
    let base64 = text
        .lines()
        .find(|line| !line.trim_start().starts_with("untrusted comment"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .ok_or_else(|| "public key file has no key line".to_owned())?;
    PublicKey::from_base64(base64)
        .map_err(|error| format!("invalid public key: {error}"))
}

fn read(path: &OsString) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", display(path)))
}

fn display(path: &OsString) -> String {
    path.to_string_lossy().into_owned()
}
