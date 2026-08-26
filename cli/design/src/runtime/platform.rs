//! Whether the host can run the bundled glibc runtime, decided before any
//! fetch so an unsupported host refuses at zero network cost.
//!
//! The classification is a pure function over two filesystem observations the
//! adapter supplies, so every host shape — including macOS and a shell-less
//! distroless image — is a unit test over injected inputs rather than a
//! container. It is **musl-first**: `/bin/sh`'s loader being musl refuses even
//! when a glibc psABI loader is also present (the Alpine + `gcompat` case),
//! because `gcompat` puts a glibc loader on a musl host that the browser still
//! cannot use. An ambiguous host fails open — it lets the later spawn decide —
//! matching every installer surveyed.

use crate::runtime::downgrade::DowngradeReason;

/// What `/bin/sh`'s `PT_INTERP` basename tells us about the host's libc.
///
/// Three-valued on purpose: `/bin/sh` is not guaranteed to exist (distroless),
/// be readable, or carry a `PT_INTERP` (a busybox-static shell on glibc), and
/// conflating "cannot tell" with either answer misclassifies a real host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellLoader {
    /// An `ld-musl-*` loader basename — positive musl evidence.
    Musl,
    /// Some other loader basename (a glibc `ld-linux-*`, say).
    Other(String),
    /// No observable loader, with the reason recorded for diagnostics.
    Unobservable(String),
}

/// The two observations the classification consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observations {
    /// Whether this is a Linux target at all — non-Linux never reaches the
    /// probe, so it is `Supported` by construction.
    pub is_linux: bool,
    /// `/bin/sh`'s loader basename.
    pub shell_loader: ShellLoader,
    /// Whether the psABI interpreter the artifact demands
    /// (`/lib64/ld-linux-x86-64.so.2` on `x86_64`, `/lib/ld-linux-aarch64.so.1`
    /// on aarch64) is present and executable.
    pub psabi_interpreter_present: bool,
}

/// Whether the bundled runtime can run here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Supported,
    Unsupported(DowngradeReason),
}

/// Classify a host from its observations, before any artifact resolution.
#[must_use]
pub fn classify(observations: &Observations) -> Support {
    if !observations.is_linux {
        return Support::Supported;
    }
    if observations.shell_loader == ShellLoader::Musl {
        // musl wins over a present glibc loader: gcompat puts one on a musl
        // host the browser still cannot use.
        return Support::Unsupported(DowngradeReason::UnsupportedPlatform);
    }
    if observations.psabi_interpreter_present {
        return Support::Supported;
    }
    // glibc (or unobservable, failing open) but the loader is not where the
    // artifact demands it — a relocated-loader host such as NixOS.
    Support::Unsupported(DowngradeReason::LoaderUnresolvable)
}

#[cfg(test)]
mod tests {
    use super::classify;
    use super::Observations;
    use super::ShellLoader;
    use super::Support;
    use crate::runtime::downgrade::DowngradeReason;

    fn linux(shell: ShellLoader, psabi: bool) -> Observations {
        Observations {
            is_linux: true,
            shell_loader: shell,
            psabi_interpreter_present: psabi,
        }
    }

    #[test]
    fn macos_is_supported_without_reaching_the_probe() {
        let observations = Observations {
            is_linux: false,
            shell_loader: ShellLoader::Unobservable("not linux".to_owned()),
            psabi_interpreter_present: false,
        };
        assert_eq!(classify(&observations), Support::Supported);
    }

    #[test]
    fn debian_glibc_is_supported() {
        let observations =
            linux(ShellLoader::Other("ld-linux-x86-64.so.2".to_owned()), true);
        assert_eq!(classify(&observations), Support::Supported);
    }

    #[test]
    fn debian_with_musl_tools_is_still_supported() {
        // musl-tools does not change /bin/sh's loader, which stays glibc's.
        let observations =
            linux(ShellLoader::Other("ld-linux-x86-64.so.2".to_owned()), true);
        assert_eq!(classify(&observations), Support::Supported);
    }

    #[test]
    fn alpine_musl_is_unsupported() {
        let observations = linux(ShellLoader::Musl, false);
        assert_eq!(
            classify(&observations),
            Support::Unsupported(DowngradeReason::UnsupportedPlatform)
        );
    }

    #[test]
    fn alpine_with_gcompat_still_refuses_despite_a_glibc_loader() {
        // gcompat provides the psABI loader, but musl must win.
        let observations = linux(ShellLoader::Musl, true);
        assert_eq!(
            classify(&observations),
            Support::Unsupported(DowngradeReason::UnsupportedPlatform)
        );
    }

    #[test]
    fn a_relocated_loader_host_emits_loader_unresolvable() {
        // NixOS: glibc basename, but the psABI path does not exist.
        let observations =
            linux(ShellLoader::Other("ld-linux-x86-64.so.2".to_owned()), false);
        assert_eq!(
            classify(&observations),
            Support::Unsupported(DowngradeReason::LoaderUnresolvable)
        );
    }

    #[test]
    fn a_shell_less_distroless_glibc_image_fails_open_to_supported() {
        let observations =
            linux(ShellLoader::Unobservable("no /bin/sh".to_owned()), true);
        assert_eq!(classify(&observations), Support::Supported);
    }
}
