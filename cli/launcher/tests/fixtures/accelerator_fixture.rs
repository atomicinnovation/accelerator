//! A stand-in sub-binary the launcher resolves and execs in tests, exposing
//! behaviours by argument. Located via `CARGO_BIN_EXE_accelerator-fixture`.
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
    // args_os so a non-UTF-8 forwarded argument does not panic.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

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

/// Print a readiness line, then block until signalled, so a test can wait for
/// the line rather than race a timer before sending SIGTERM.
fn block_until_signalled() -> ! {
    println!("{READY_SENTINEL}");
    // Flush before blocking so the reader unblocks (stdout is pipe-buffered).
    let _ = std::io::stdout().flush();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// `write-args-to <file> <arg>...` — write the args after the destination,
/// NUL-separated raw bytes, so a test can assert non-UTF-8 args survived exec.
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
