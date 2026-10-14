use std::os::fd::AsRawFd as _;
use std::os::fd::RawFd;

use rustix::fs::Dir;
use rustix::fs::Mode;
use rustix::fs::OFlags;

/// Every open descriptor above stderr, read from `/dev/fd`, except the one the
/// listing reads through.
///
/// # Errors
///
/// An [`std::io::Error`] if `/dev/fd` cannot be opened or read.
pub fn descriptors_above_stdio() -> std::io::Result<Vec<RawFd>> {
    let listing = rustix::fs::open(
        "/dev/fd",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let own = listing.as_raw_fd();
    // `Dir::read_from` would open a second descriptor, listed and not skipped.
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

/// Close every descriptor this process inherited above stderr, as a detached
/// daemon does before taking its own resources.
///
/// # Safety
///
/// No other thread may be running, and nothing in the process may own a
/// descriptor above stderr: each one is closed by number, behind the back of
/// any `OwnedFd` or `File` that holds it. Call it first thing in `main`.
///
/// # Errors
///
/// An [`std::io::Error`] if `/dev/fd` cannot be listed; nothing is closed.
pub unsafe fn close_inherited_descriptors() -> std::io::Result<()> {
    for descriptor in descriptors_above_stdio()? {
        // SAFETY: the caller guarantees nothing owns the descriptor and no
        // other thread can close it first; it was just listed, so it is open.
        unsafe { rustix::io::close(descriptor) };
    }
    Ok(())
}
