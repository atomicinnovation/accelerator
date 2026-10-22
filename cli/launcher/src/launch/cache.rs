//! The `accelerator cache` built-in: verify, repair, ensure and prune over the
//! sealed tree artifacts.
//!
//! Every verb validates its `<name>` against the launcher's compiled-in
//! artifact set before touching the filesystem, so no path is ever constructed
//! from an unrecognised token and `verify`/`prune` stay offline.

use std::io::Write;
use std::time::Duration;

use crate::launch::core::tree::{
    Discrepancy, MaterialiseTree as _, TreeError, TreeReport, VerifyTree as _,
};
use crate::launch::core::ResolutionError;
use crate::launch::inbound::cli::CacheAction;
use crate::launch::outbound::resolve::retention::root_prune::{
    PruneOutcome, PruneReport,
};
use crate::launch::outbound::resolve::retention::root_retention::RootRetention;
use crate::launch::outbound::resolve::tree::{pins, TreeResolver};

/// Run one `cache` verb against `resolver`, or `retention` for `prune`,
/// writing results to `out` and what a prune failed to do to `err`.
///
/// # Errors
///
/// A [`kernel::Error`] for an unrecognised artifact name (a refusal), a
/// resolution failure that is not a recoverable miss, or a prune that was
/// refused or did not finish.
pub fn run(
    action: &CacheAction,
    resolver: &TreeResolver<'_>,
    retention: &dyn RootRetention,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), kernel::Error> {
    match action {
        CacheAction::Verify { name } => verify(resolver, name.as_deref(), out),
        CacheAction::Repair { name, force } => {
            repair(resolver, name.as_deref(), *force, out)
        }
        CacheAction::Ensure { names } => ensure(resolver, names, out),
        CacheAction::Prune { older_than } => {
            prune(retention, *older_than, out, err)
        }
    }
}

/// The artifacts a verb operates on: the one named, validated against the
/// compiled-in set, or every known artifact when none is named.
fn targets(name: Option<&str>) -> Result<Vec<String>, kernel::Error> {
    let Some(name) = name else {
        return Ok(pins::artifact_names()
            .into_iter()
            .map(str::to_owned)
            .collect());
    };
    if pins::is_known_artifact(name) {
        Ok(vec![name.to_owned()])
    } else {
        Err(kernel::Error::Refusal(format!(
            "unknown tree artifact '{name}'"
        )))
    }
}

fn verify(
    resolver: &TreeResolver<'_>,
    name: Option<&str>,
    out: &mut dyn Write,
) -> Result<(), kernel::Error> {
    let mut all_sound = true;
    for artifact in targets(name)? {
        let report = resolver.verify(&artifact)?;
        all_sound &= report.is_sound();
        render_report(&report, out);
    }
    if all_sound {
        Ok(())
    } else {
        Err(kernel::Error::Failed(
            "one or more trees failed verification; run \
             `accelerator cache repair`"
                .to_owned(),
        ))
    }
}

fn repair(
    resolver: &TreeResolver<'_>,
    name: Option<&str>,
    force: bool,
    out: &mut dyn Write,
) -> Result<(), kernel::Error> {
    for artifact in targets(name)? {
        let sealed = resolver.repair(&artifact, force)?;
        let _ = writeln!(out, "{artifact}\t{}", sealed.path.display());
    }
    Ok(())
}

fn ensure(
    resolver: &TreeResolver<'_>,
    names: &[String],
    out: &mut dyn Write,
) -> Result<(), kernel::Error> {
    for name in names {
        if !pins::is_known_artifact(name) {
            return Err(kernel::Error::Refusal(format!(
                "unknown tree artifact '{name}'"
            )));
        }
    }
    // Success lines go to `out` (stdout); a materialise failure returns a
    // structured envelope whose Display the caller renders to stderr, so the
    // executor maps a cause token rather than parsing prose.
    for name in names {
        match resolver.materialise(name) {
            Ok(sealed) => {
                // The caller holds the lease itself, so the lease path is
                // printed alongside the resolved tree path.
                let _ = writeln!(
                    out,
                    "{name}\t{}\t{}",
                    sealed.path.display(),
                    sealed.lease_path.display()
                );
            }
            Err(error) => {
                return Err(kernel::Error::Failed(ensure_envelope(
                    name, &error,
                )));
            }
        }
    }
    Ok(())
}

/// The failure envelope for a single artifact: the machine-readable cause the
/// executor maps, the artifact, and the human-readable detail.
fn ensure_envelope(artifact: &str, error: &TreeError) -> String {
    format!(
        r#"{{"error":"ensure-failed","cause":"{}","artifact":"{}","message":"{}"}}"#,
        error.cause().as_str(),
        escape(artifact),
        escape(&error.to_string()),
    )
}

/// JSON string escaping over the only metacharacters a path or detail carries.
fn escape(raw: &str) -> String {
    raw.replace('\\', r"\\").replace('"', "\\\"")
}

fn prune(
    retention: &dyn RootRetention,
    older_than: Option<u64>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), kernel::Error> {
    match retention.prune(older_than.map(Duration::from_secs)) {
        PruneOutcome::Ran(report) => render_prune_report(&report, out, err),
        PruneOutcome::Refused(unsafe_root) => {
            Err(kernel::Error::Failed(format!("not pruning: {unsafe_root}")))
        }
        PruneOutcome::Unavailable { detail } => {
            Err(ResolutionError::CacheRootUnavailable { detail }.into())
        }
    }
}

/// An unreadable claims directory alone succeeds: it only kept entries.
fn render_prune_report(
    report: &PruneReport,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), kernel::Error> {
    let _ = writeln!(out, "reclaimed {} entries", report.reclaimed);
    for failure in &report.lock_failures {
        let _ = writeln!(
            err,
            "could not lock {}: {}",
            failure.lock.display(),
            failure.detail
        );
    }
    for unreadable in &report.unreadable_claims {
        let _ = writeln!(
            err,
            "claims unreadable ({}) in {}; kept every versioned entry",
            unreadable.kind,
            unreadable.directory.display()
        );
    }
    for failure in &report.removal_failures {
        let _ = writeln!(
            err,
            "could not remove {}: {}",
            failure.path.display(),
            failure.kind
        );
    }
    if report.lock_failures.is_empty() && report.removal_failures.is_empty() {
        Ok(())
    } else {
        Err(kernel::Error::Failed(
            "the prune did not finish; see the failures above".to_owned(),
        ))
    }
}

fn render_report(report: &TreeReport, out: &mut dyn Write) {
    if report.is_sound() {
        let _ = writeln!(out, "{}\tok", report.artifact);
        return;
    }
    for (path, discrepancy) in &report.findings {
        let _ = writeln!(
            out,
            "{}\t{}\t{}",
            report.artifact,
            describe(discrepancy),
            path
        );
    }
}

const fn describe(discrepancy: &Discrepancy) -> &'static str {
    match discrepancy {
        Discrepancy::Missing => "missing",
        Discrepancy::Unexpected => "unexpected",
        Discrepancy::Size { .. } => "size",
        Discrepancy::Mode { .. } => "mode",
        Discrepancy::Digest => "digest",
        Discrepancy::LinkTarget { .. } => "link-target",
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_envelope;
    use crate::launch::core::tree::TreeError;

    #[test]
    fn a_disk_shortfall_renders_a_cause_keyed_envelope() {
        assert_eq!(
            ensure_envelope(
                "driver",
                &TreeError::DiskShortfall {
                    needed: 600,
                    available: 12,
                },
            ),
            r#"{"error":"ensure-failed","cause":"disk-shortfall","artifact":"driver","message":"materialising needs 600 bytes and 12 are free"}"#
        );
    }

    #[test]
    fn an_unreachable_archive_is_named_as_such() {
        let envelope = ensure_envelope(
            "browser",
            &TreeError::Unreachable {
                detail: "connection refused".to_owned(),
            },
        );
        assert!(envelope.contains(r#""cause":"unreachable""#), "{envelope}");
        assert!(envelope.contains(r#""artifact":"browser""#), "{envelope}");
    }

    #[test]
    fn a_metacharacter_in_the_detail_is_escaped() {
        let envelope = ensure_envelope(
            "driver",
            &TreeError::Extraction {
                detail: r#"a "quoted" member"#.to_owned(),
            },
        );
        assert!(envelope.contains(r#"\"quoted\""#), "{envelope}");
    }
}
