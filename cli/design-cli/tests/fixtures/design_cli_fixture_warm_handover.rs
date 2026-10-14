//! The launcher's warm handover, reproduced in a process whose only open
//! descriptors are 0-2: it takes the driver's and the browser's leases, exports
//! both trees and `exec`s the executor.
//!
//! Usage: `<executor> <driver tree> <driver lease> <browser tree>
//! <browser lease> [executor arguments...]`

use std::ffi::OsString;
use std::os::fd::AsRawFd as _;
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::Command;
use std::process::ExitCode;

use lease::LeaseError;
use lease::SharedLease;
use rustix::fs::Dir;
use rustix::fs::Mode;
use rustix::fs::OFlags;

const USAGE: u8 = 64;
const DESCRIPTORS_UNLISTABLE: u8 = 70;
const IDENTITY_DESCRIPTOR_STILL_OPEN: u8 = 71;
const LEASE_UNHELD: u8 = 72;
const EXEC_FAILED: u8 = 73;

const IDENTITY_DESCRIPTOR: RawFd = 3;

fn main() -> ExitCode {
    let arguments: Vec<OsString> = std::env::args_os().skip(1).collect();
    let [executor, driver_tree, driver_lease, browser_tree, browser_lease, forwarded @ ..] =
        arguments.as_slice()
    else {
        eprintln!(
            "usage: <executor> <driver tree> <driver lease> <browser tree> \
             <browser lease> [executor arguments...]"
        );
        return ExitCode::from(USAGE);
    };

    if let Err(error) = close_inherited_descriptors() {
        eprintln!(
            "cannot list /dev/fd to close inherited descriptors: {error}"
        );
        return ExitCode::from(DESCRIPTORS_UNLISTABLE);
    }
    match descriptors_above_stdio() {
        Ok(open) if open.contains(&IDENTITY_DESCRIPTOR) => {
            eprintln!("fd {IDENTITY_DESCRIPTOR} is still open after closing");
            return ExitCode::from(IDENTITY_DESCRIPTOR_STILL_OPEN);
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("cannot list /dev/fd to check fd 3: {error}");
            return ExitCode::from(DESCRIPTORS_UNLISTABLE);
        }
    }

    let _leases = match hold_in_launcher_order(driver_lease, browser_lease) {
        Ok(leases) => leases,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(LEASE_UNHELD);
        }
    };

    let error = Command::new(executor)
        .args(forwarded)
        .env("ACCELERATOR_TREE_DRIVER", driver_tree)
        .env("ACCELERATOR_TREE_BROWSER", browser_tree)
        .exec();
    eprintln!("cannot exec {}: {error}", executor.to_string_lossy());
    ExitCode::from(EXEC_FAILED)
}

fn hold_in_launcher_order(
    driver_lease: &OsString,
    browser_lease: &OsString,
) -> Result<[SharedLease; 2], LeaseError> {
    Ok([
        lease::hold_shared_lease(Path::new(driver_lease))?,
        lease::hold_shared_lease(Path::new(browser_lease))?,
    ])
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
