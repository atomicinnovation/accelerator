//! A stand-in for a detached design daemon: it leads its own session, closes
//! every descriptor it inherited, takes a shared lease on each path it is
//! given, writes `ready` and sleeps.
//!
//! Usage: `<lease path>...`

use std::io::Write as _;
use std::process::ExitCode;
use std::time::Duration;

const SESSION_UNAVAILABLE: u8 = 69;
const DESCRIPTORS_UNLISTABLE: u8 = 70;
const LEASE_UNHELD: u8 = 72;
const READY_UNWRITABLE: u8 = 74;

const LIFETIME: Duration = Duration::from_secs(300);

fn main() -> ExitCode {
    if let Err(error) = rustix::process::setsid() {
        eprintln!("cannot lead a session: {error}");
        return ExitCode::from(SESSION_UNAVAILABLE);
    }
    // SAFETY: single-threaded, and nothing is opened before this point.
    if let Err(error) = unsafe { process::close_inherited_descriptors() } {
        eprintln!(
            "cannot list /dev/fd to close inherited descriptors: {error}"
        );
        return ExitCode::from(DESCRIPTORS_UNLISTABLE);
    }

    let mut leases = Vec::new();
    for path in std::env::args_os().skip(1) {
        match lease::hold_shared_lease(path.as_ref()) {
            Ok(lease) => leases.push(lease),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(LEASE_UNHELD);
            }
        }
    }

    let mut stdout = std::io::stdout();
    if writeln!(stdout, "ready")
        .and_then(|()| stdout.flush())
        .is_err()
    {
        return ExitCode::from(READY_UNWRITABLE);
    }
    std::thread::sleep(LIFETIME);
    drop(leases);
    ExitCode::SUCCESS
}
