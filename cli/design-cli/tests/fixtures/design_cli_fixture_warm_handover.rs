//! The launcher's warm handover, reproduced in a process whose only open
//! descriptors are 0-2: it takes the driver's and the browser's leases, exports
//! both trees and `exec`s the executor.
//!
//! Usage: `<executor> <driver tree> <driver lease> <browser tree>
//! <browser lease> [executor arguments...]`

use std::ffi::OsString;
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt as _;
use std::path::Path;
use std::process::Command;
use std::process::ExitCode;

use lease::LeaseError;
use lease::SharedLease;

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

    // SAFETY: single-threaded, and nothing is opened before this point.
    if let Err(error) = unsafe { process::close_inherited_descriptors() } {
        eprintln!(
            "cannot list /dev/fd to close inherited descriptors: {error}"
        );
        return ExitCode::from(DESCRIPTORS_UNLISTABLE);
    }
    match process::descriptors_above_stdio() {
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
