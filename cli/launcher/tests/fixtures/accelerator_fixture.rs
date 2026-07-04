//! A stand-in "sub-binary" the launcher resolves and execs in tests, exposing
//! behaviours by argument so exit-code/signal propagation, argument forwarding,
//! and `--help` delegation can be exercised. Located via
//! `CARGO_BIN_EXE_accelerator-fixture`; the release staging (0165) copies only
//! the `accelerator` binary, so it never ships.
//!
//! Restriction lints are allowed crate-wide: a CLI stub legitimately calls
//! `process::exit` and prints to stdout.
#![allow(
    clippy::exit,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::restriction
)]

use std::ffi::OsString;
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::process;

const HELP_SENTINEL: &str = "ACCELERATOR_FIXTURE_HELP_SENTINEL";
const READY_SENTINEL: &str = "ACCELERATOR_FIXTURE_READY";

fn main() {
    // args_os, not args: a non-UTF-8 forwarded argument must not panic here.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    // `--help` delegation: whatever else was asked, a forwarded --help makes the
    // child emit its own help — a sentinel only this fixture prints.
    if args.iter().any(|arg| arg == "--help") {
        println!("{HELP_SENTINEL}");
        return;
    }

    match args.first().and_then(|arg| arg.to_str()) {
        Some("exit-42") => process::exit(42),
        Some("block-on-sigterm") => block_until_signalled(),
        Some("print-help-sentinel") => println!("{HELP_SENTINEL}"),
        Some("write-args-to") => write_forwarded_args(&args),
        other => {
            eprintln!("accelerator-fixture: unknown behaviour {other:?}");
            process::exit(1);
        }
    }
}

/// Print a readiness line (so a test can wait for it rather than guess timing),
/// then block forever — the caller kills us with SIGTERM and observes 128+15.
fn block_until_signalled() -> ! {
    println!("{READY_SENTINEL}");
    // Flush so the reader unblocks before we sleep; stdout is line-buffered to a
    // pipe, so an explicit flush avoids a deadlock on the handshake.
    let _ = std::io::stdout().flush();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// `write-args-to <file> <arg>...` — write the args AFTER the destination path,
/// NUL-separated, as raw bytes, so a test can assert non-UTF-8 arguments
/// survived exec verbatim.
fn write_forwarded_args(args: &[OsString]) {
    let Some(destination) = args.get(1) else {
        eprintln!("accelerator-fixture: write-args-to needs a destination");
        process::exit(1);
    };
    let mut bytes = Vec::new();
    for arg in &args[2..] {
        bytes.extend_from_slice(arg.as_bytes());
        bytes.push(0);
    }
    if let Err(error) = std::fs::write(destination, bytes) {
        eprintln!("accelerator-fixture: write failed: {error}");
        process::exit(1);
    }
}
