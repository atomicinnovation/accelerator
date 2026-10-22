//! Who the effective user and their primary group are, by name, read from the
//! host's account databases.

/// The effective user's login and their primary group's name and listed
/// members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accounts {
    pub login: String,
    pub group_name: String,
    pub members: Vec<String>,
}

/// One way of resolving the effective user's accounts; `None` when it has no
/// entry for either.
pub trait AccountLookup {
    fn accounts(&self, uid: u32, gid: u32) -> Option<Accounts>;
}

/// The C library's `getpwuid_r` and `getgrgid_r`.
pub struct LibcLookup;

impl AccountLookup for LibcLookup {
    fn accounts(&self, uid: u32, gid: u32) -> Option<Accounts> {
        let login = libc_db::login(uid)?;
        let (group_name, members) = libc_db::group(gid)?;
        Some(Accounts {
            login,
            group_name,
            members,
        })
    }
}

#[cfg(unix)]
mod libc_db {
    use std::ffi::{c_char, CStr};
    use std::mem::MaybeUninit;
    use std::ptr;

    /// Enough for any account database entry seen in practice; a lookup still
    /// asking for more is treated as having no entry.
    const MAX_BUFFER: usize = 1 << 20;
    const INITIAL_BUFFER: usize = 1 << 10;

    enum Attempt<R> {
        Found(R),
        Absent,
        TooSmall,
    }

    pub fn login(uid: u32) -> Option<String> {
        with_growing_buffer(|buffer| {
            let mut entry = MaybeUninit::<libc::passwd>::uninit();
            let mut found: *mut libc::passwd = ptr::null_mut();
            // SAFETY: every pointer is valid for the call, and `buffer` is
            // passed with its own length, as `getpwuid_r` requires.
            let status = unsafe {
                libc::getpwuid_r(
                    uid,
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &raw mut found,
                )
            };
            attempt(status, found, |found| {
                // SAFETY: `found` points at `entry`, whose strings live in
                // `buffer`; both outlive this read.
                unsafe { owned((*found).pw_name) }
            })
        })
    }

    pub fn group(gid: u32) -> Option<(String, Vec<String>)> {
        with_growing_buffer(|buffer| {
            let mut entry = MaybeUninit::<libc::group>::uninit();
            let mut found: *mut libc::group = ptr::null_mut();
            // SAFETY: as for `getpwuid_r` in `login`.
            let status = unsafe {
                libc::getgrgid_r(
                    gid,
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &raw mut found,
                )
            };
            attempt(status, found, |found| {
                // SAFETY: as in `login`; `gr_mem` is a null-terminated array
                // of strings inside the same buffer.
                unsafe {
                    let name = owned((*found).gr_name)?;
                    let mut members = Vec::new();
                    let mut member = (*found).gr_mem;
                    while !member.is_null() && !(*member).is_null() {
                        members.push(owned(*member)?);
                        member = member.add(1);
                    }
                    Some((name, members))
                }
            })
        })
    }

    fn attempt<P, R>(
        status: i32,
        found: *mut P,
        read: impl FnOnce(*mut P) -> Option<R>,
    ) -> Attempt<R> {
        match status {
            libc::ERANGE => Attempt::TooSmall,
            0 if !found.is_null() => {
                read(found).map_or(Attempt::Absent, Attempt::Found)
            }
            _ => Attempt::Absent,
        }
    }

    fn with_growing_buffer<R>(
        lookup: impl Fn(&mut [c_char]) -> Attempt<R>,
    ) -> Option<R> {
        let mut size = INITIAL_BUFFER;
        loop {
            let mut buffer = vec![0; size];
            match lookup(&mut buffer) {
                Attempt::Found(found) => return Some(found),
                Attempt::TooSmall if size < MAX_BUFFER => size *= 2,
                Attempt::TooSmall | Attempt::Absent => return None,
            }
        }
    }

    /// # Safety
    ///
    /// `pointer` is null or a live NUL-terminated string.
    unsafe fn owned(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live NUL-terminated string.
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

#[cfg(not(unix))]
mod libc_db {
    pub const fn login(_uid: u32) -> Option<String> {
        None
    }

    pub const fn group(_gid: u32) -> Option<(String, Vec<String>)> {
        None
    }
}
