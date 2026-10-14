//! A stand-in for a detached design daemon: it leads its own session, closes
//! every descriptor it inherited, takes a shared lease on each path it is
//! given, writes `ready` and sleeps.
//!
//! Usage: `<lease path>...`

use std::io::Write as _;
use std::os::fd::AsRawFd as _;
use std::os::fd::RawFd;
use std::process::ExitCode;
use std::time::Duration;

use rustix::fs::Dir;
use rustix::fs::Mode;
use rustix::fs::OFlags;

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
    if let Err(error) = close_inherited_descriptors() {
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

fn close_inherited_descriptors() -> rustix::io::Result<()> {
    for descriptor in descriptors_above_stdio()? {
        // SAFETY: the descriptor was listed in /dev/fd, it is not the
        // listing's own, and this process is single-threaded, so it is open
        // and nothing else can close it first.
        unsafe { rustix::io::close(descriptor) };
    }
    Ok(())
}

/// Every open descriptor above stderr, except the one the listing reads
/// through. `Dir::new` reads through the descriptor it is given, where
/// `Dir::read_from` would open a second one that is listed and not skipped.
fn descriptors_above_stdio() -> rustix::io::Result<Vec<RawFd>> {
    let listing = rustix::fs::open(
        "/dev/fd",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let own = listing.as_raw_fd();
    let mut open = Vec::new();
    for entry in Dir::new(listing)? {
        let Some(descriptor) = entry?
            .file_name()
            .to_str()
            .ok()
            .and_then(|name| name.parse::<RawFd>().ok())
        else {
            continue;
        };
        if descriptor > 2 && descriptor != own {
            open.push(descriptor);
        }
    }
    Ok(open)
}
