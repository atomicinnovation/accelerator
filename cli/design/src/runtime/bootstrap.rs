//! Classify why a daemon spawn failed, from its bootstrap log.
//!
//! Two host conditions cannot be predicted from this static-musl binary — a
//! glibc too old for the bundled browser, and a missing shared library — so
//! they are read out of the loader's own error output after the spawn fails.
//!
//! The log is untrusted input: a `design.browser_path` wrapper, an ambient
//! `NODE_OPTIONS`, or renderer output while crawling could emit a marker
//! substring. So a line classifies only when it carries the loader's whole
//! error shape (both the prefix marker and the trailing "not found" / "cannot
//! open" marker), and every extracted token is validated before it can reach a
//! remediation string an agent reads. Anything else falls back to
//! `executor-ping-failed` rather than guessing.

use crate::runtime::downgrade::DowngradeReason;

/// Only the start of a failed start is scanned; the loader's error is first.
const MAX_SCANNED_LINES: usize = 40;
/// A soname beyond this is rejected rather than interpolated.
const MAX_SONAME_LEN: usize = 64;

/// A classified spawn failure and the token, if any, to name in the remedy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnDiagnosis {
    pub reason: DowngradeReason,
    pub detail: Option<String>,
}

/// Classify a failed daemon's bootstrap log into a downgrade reason.
#[must_use]
pub fn classify_bootstrap_log(lines: &[&str]) -> SpawnDiagnosis {
    for line in lines.iter().take(MAX_SCANNED_LINES) {
        if let Some(version) = glibc_version(line) {
            return SpawnDiagnosis {
                reason: DowngradeReason::GlibcTooOld,
                detail: Some(version),
            };
        }
        if let Some(soname) = missing_soname(line) {
            return SpawnDiagnosis {
                reason: DowngradeReason::RuntimeLibrariesMissing,
                detail: Some(soname),
            };
        }
    }
    SpawnDiagnosis {
        reason: DowngradeReason::ExecutorPingFailed,
        detail: None,
    }
}

fn glibc_version(line: &str) -> Option<String> {
    const PREFIX: &str = "version `GLIBC_";
    let start = line.find(PREFIX)? + PREFIX.len();
    let rest = &line[start..];
    let end = rest.find('\'')?;
    let version = &rest[..end];
    if !rest[end..].contains("not found") {
        return None;
    }
    is_glibc_version(version).then(|| version.to_owned())
}

fn is_glibc_version(version: &str) -> bool {
    version.contains('.')
        && version.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
        })
}

fn missing_soname(line: &str) -> Option<String> {
    const PREFIX: &str = "error while loading shared libraries: ";
    if !line.contains("cannot open shared object file") {
        return None;
    }
    let start = line.find(PREFIX)? + PREFIX.len();
    let rest = &line[start..];
    let end = rest.find(':')?;
    let soname = &rest[..end];
    is_valid_soname(soname).then(|| soname.to_owned())
}

fn is_valid_soname(soname: &str) -> bool {
    !soname.is_empty()
        && soname.len() <= MAX_SONAME_LEN
        && soname.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-')
        })
}

#[cfg(test)]
mod tests {
    use super::classify_bootstrap_log;
    use crate::runtime::downgrade::DowngradeReason;

    #[test]
    fn a_glibc_version_error_extracts_the_version() {
        let log = [
            "starting daemon",
            "/cache/node: /lib/x86_64-linux-gnu/libm.so.6: version \
             `GLIBC_2.34' not found (required by /cache/node)",
        ];
        let diagnosis = classify_bootstrap_log(&log);
        assert_eq!(diagnosis.reason, DowngradeReason::GlibcTooOld);
        assert_eq!(diagnosis.detail.as_deref(), Some("2.34"));
    }

    #[test]
    fn a_missing_library_error_names_the_soname() {
        let log = [
            "chrome-headless-shell: error while loading shared libraries: \
             libnss3.so: cannot open shared object file: No such file",
        ];
        let diagnosis = classify_bootstrap_log(&log);
        assert_eq!(diagnosis.reason, DowngradeReason::RuntimeLibrariesMissing);
        assert_eq!(diagnosis.detail.as_deref(), Some("libnss3.so"));
    }

    #[test]
    fn an_unrecognised_failure_falls_back_to_executor_ping_failed() {
        let log = ["daemon exited with code 1", "no idea why"];
        let diagnosis = classify_bootstrap_log(&log);
        assert_eq!(diagnosis.reason, DowngradeReason::ExecutorPingFailed);
        assert_eq!(diagnosis.detail, None);
    }

    #[test]
    fn a_glibc_marker_without_the_not_found_suffix_does_not_classify() {
        // Renderer output echoing the marker, not the loader's own error.
        let log = ["page text mentioning version `GLIBC_9.99' in passing"];
        assert_eq!(
            classify_bootstrap_log(&log).reason,
            DowngradeReason::ExecutorPingFailed
        );
    }

    #[test]
    fn a_soname_marker_without_the_full_loader_shape_does_not_classify() {
        let log = ["something about error while loading shared libraries: x"];
        assert_eq!(
            classify_bootstrap_log(&log).reason,
            DowngradeReason::ExecutorPingFailed
        );
    }

    #[test]
    fn a_metacharacter_bearing_soname_is_rejected() {
        let log = ["error while loading shared libraries: lib$(rm -rf ~).so: \
             cannot open shared object file"];
        // The soname carries `$`, `(` and a space, so it never reaches a remedy.
        assert_eq!(
            classify_bootstrap_log(&log).reason,
            DowngradeReason::ExecutorPingFailed
        );
    }

    #[test]
    fn an_over_long_soname_is_rejected() {
        let soname = "a".repeat(200);
        let line = format!(
            "error while loading shared libraries: {soname}.so: \
             cannot open shared object file"
        );
        assert_eq!(
            classify_bootstrap_log(&[line.as_str()]).reason,
            DowngradeReason::ExecutorPingFailed
        );
    }
}
