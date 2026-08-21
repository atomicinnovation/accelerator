//! Host observations feeding the pre-fetch platform classification.
//!
//! The classification itself is a pure domain function; this gathers the two
//! filesystem observations it consumes. On a non-Linux host the probe never
//! runs — `is_linux` is `false` and the domain returns `Supported` by
//! construction — so the real reads are compiled only for Linux targets.
//!
//! `/bin/sh`'s loader is read from its `PT_INTERP`, whose basename is the
//! libc discriminator: an `ld-musl-*` basename is positive musl evidence
//! wherever the loader lives, so a location-based test (which would
//! misclassify `NixOS`'s relocated glibc loader) is avoided.

use design::runtime::platform::Observations;
use design::runtime::platform::ShellLoader;

/// Gather the host observations the domain classification consumes.
#[must_use]
pub fn observe() -> Observations {
    observe_impl()
}

/// The libc discriminator carried by a loader path's basename.
#[cfg(any(target_os = "linux", test))]
fn classify_interp(interp: &str) -> ShellLoader {
    let basename = interp.rsplit('/').next().unwrap_or(interp);
    if basename.starts_with("ld-musl-") {
        ShellLoader::Musl
    } else {
        ShellLoader::Other(basename.to_owned())
    }
}

/// Extract an ELF image's `PT_INTERP` path, or `None` when it carries none or
/// cannot be parsed. Handles 32- and 64-bit images of either endianness so the
/// parser is exercised on the build host, not only on the Linux target.
#[cfg(any(target_os = "linux", test))]
fn elf_interp(bytes: &[u8]) -> Option<String> {
    const PT_INTERP: u32 = 3;

    if bytes.get(0..4)? != b"\x7fELF" {
        return None;
    }
    let is_64 = match bytes.get(4)? {
        1 => false,
        2 => true,
        _ => return None,
    };
    let little_endian = match bytes.get(5)? {
        1 => true,
        2 => false,
        _ => return None,
    };

    let read_u16 = |offset: usize| -> Option<u16> {
        let slice = bytes.get(offset..offset + 2)?.try_into().ok()?;
        Some(if little_endian {
            u16::from_le_bytes(slice)
        } else {
            u16::from_be_bytes(slice)
        })
    };
    let read_u32 = |offset: usize| -> Option<u32> {
        let slice = bytes.get(offset..offset + 4)?.try_into().ok()?;
        Some(if little_endian {
            u32::from_le_bytes(slice)
        } else {
            u32::from_be_bytes(slice)
        })
    };
    let read_u64 = |offset: usize| -> Option<u64> {
        let slice = bytes.get(offset..offset + 8)?.try_into().ok()?;
        Some(if little_endian {
            u64::from_le_bytes(slice)
        } else {
            u64::from_be_bytes(slice)
        })
    };

    let (phoff, phentsize_off, phnum_off) = if is_64 {
        (0x20, 0x36, 0x38)
    } else {
        (0x1c, 0x2a, 0x2c)
    };
    let phoff = usize::try_from(if is_64 {
        read_u64(phoff)?
    } else {
        u64::from(read_u32(phoff)?)
    })
    .ok()?;
    let phentsize = usize::from(read_u16(phentsize_off)?);
    let phnum = usize::from(read_u16(phnum_off)?);

    for index in 0..phnum {
        let header = phoff.checked_add(index.checked_mul(phentsize)?)?;
        if read_u32(header)? != PT_INTERP {
            continue;
        }
        let (offset, filesz) = if is_64 {
            (
                usize::try_from(read_u64(header + 0x08)?).ok()?,
                usize::try_from(read_u64(header + 0x20)?).ok()?,
            )
        } else {
            (
                usize::try_from(read_u32(header + 0x04)?).ok()?,
                usize::try_from(read_u32(header + 0x10)?).ok()?,
            )
        };
        let segment = bytes.get(offset..offset.checked_add(filesz)?)?;
        let text = segment.split(|byte| *byte == 0).next()?;
        return std::str::from_utf8(text).ok().map(str::to_owned);
    }
    None
}

#[cfg(target_os = "linux")]
fn observe_impl() -> Observations {
    Observations {
        is_linux: true,
        shell_loader: shell_loader(),
        psabi_interpreter_present: psabi_interpreter_present(),
    }
}

#[cfg(not(target_os = "linux"))]
fn observe_impl() -> Observations {
    Observations {
        is_linux: false,
        shell_loader: ShellLoader::Unobservable("not a Linux host".to_owned()),
        psabi_interpreter_present: false,
    }
}

#[cfg(target_os = "linux")]
fn shell_loader() -> ShellLoader {
    match std::fs::read("/bin/sh") {
        Err(error) => {
            ShellLoader::Unobservable(format!("cannot read /bin/sh: {error}"))
        }
        Ok(bytes) => elf_interp(&bytes).map_or_else(
            || ShellLoader::Unobservable("/bin/sh has no PT_INTERP".to_owned()),
            |interp| classify_interp(&interp),
        ),
    }
}

/// The psABI interpreter the artifact demands, noting the `x86-64`/`x86_64`
/// spelling asymmetry against the target-arch spelling.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const PSABI_INTERPRETER: &str = "/lib64/ld-linux-x86-64.so.2";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const PSABI_INTERPRETER: &str = "/lib/ld-linux-aarch64.so.1";

#[cfg(target_os = "linux")]
fn psabi_interpreter_present() -> bool {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::metadata(PSABI_INTERPRETER).is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    })
}

#[cfg(test)]
mod tests {
    use super::classify_interp;
    use super::elf_interp;
    use super::observe;
    use design::runtime::platform::ShellLoader;

    type TestError = Box<dyn std::error::Error>;

    /// A minimal 64-bit little-endian ELF carrying a single `PT_INTERP`.
    fn elf64_le_with_interp(interp: &str) -> Result<Vec<u8>, TestError> {
        const EHDR: usize = 64;
        const PHENT: usize = 56;
        let interp_offset = EHDR + PHENT;

        let mut bytes = vec![0u8; interp_offset + interp.len() + 1];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2; // 64-bit
        bytes[5] = 1; // little-endian
        bytes[6] = 1; // version
        bytes[0x20..0x28].copy_from_slice(&u64::try_from(EHDR)?.to_le_bytes()); // e_phoff
        bytes[0x36..0x38].copy_from_slice(&u16::try_from(PHENT)?.to_le_bytes()); // e_phentsize
        bytes[0x38..0x3a].copy_from_slice(&1u16.to_le_bytes()); // e_phnum

        let header = EHDR;
        bytes[header..header + 4].copy_from_slice(&3u32.to_le_bytes()); // PT_INTERP
        bytes[header + 0x08..header + 0x10]
            .copy_from_slice(&u64::try_from(interp_offset)?.to_le_bytes());
        bytes[header + 0x20..header + 0x28]
            .copy_from_slice(&u64::try_from(interp.len() + 1)?.to_le_bytes());
        bytes[interp_offset..interp_offset + interp.len()]
            .copy_from_slice(interp.as_bytes());
        Ok(bytes)
    }

    #[test]
    fn a_musl_loader_basename_classifies_as_musl() {
        assert_eq!(
            classify_interp("/lib/ld-musl-x86_64.so.1"),
            ShellLoader::Musl
        );
    }

    #[test]
    fn a_relocated_glibc_loader_keeps_its_basename() {
        // NixOS keeps glibc's loader under /nix/store; the basename is what
        // the classifier keys on, not the location.
        assert_eq!(
            classify_interp(
                "/nix/store/abc-glibc-2.39/lib/ld-linux-x86-64.so.2"
            ),
            ShellLoader::Other("ld-linux-x86-64.so.2".to_owned())
        );
    }

    #[test]
    fn an_elf_interp_segment_is_extracted() -> Result<(), TestError> {
        let elf = elf64_le_with_interp("/lib/ld-musl-aarch64.so.1")?;
        assert_eq!(
            elf_interp(&elf).as_deref(),
            Some("/lib/ld-musl-aarch64.so.1")
        );
        assert_eq!(
            elf_interp(&elf).map(|interp| classify_interp(&interp)),
            Some(ShellLoader::Musl)
        );
        Ok(())
    }

    #[test]
    fn a_static_elf_with_no_interp_segment_yields_none() -> Result<(), TestError>
    {
        let mut elf = elf64_le_with_interp("/unused")?;
        // Zero the single program header's type so no PT_INTERP remains.
        elf[64..68].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(elf_interp(&elf), None);
        Ok(())
    }

    #[test]
    fn non_elf_bytes_yield_none() {
        assert_eq!(elf_interp(b"#!/bin/sh\n"), None);
        assert_eq!(elf_interp(b""), None);
    }

    #[test]
    fn observe_reports_a_non_linux_host_as_such() {
        // On the build host (macOS) the probe never runs.
        if !cfg!(target_os = "linux") {
            assert!(!observe().is_linux);
        }
    }
}
