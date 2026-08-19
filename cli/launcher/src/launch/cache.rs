//! The `accelerator cache` built-in: verify, repair, ensure and prune over the
//! sealed tree artifacts.
//!
//! Every verb validates its `<name>` against the launcher's compiled-in
//! artifact set before touching the filesystem, so no path is ever constructed
//! from an unrecognised token and `verify`/`prune` stay offline.

use std::io::Write;

use crate::launch::core::tree::{
    Discrepancy, MaterialiseTree as _, TreeReport, VerifyTree as _,
};
use crate::launch::inbound::cli::CacheAction;
use crate::launch::outbound::resolve::tree::{pins, TreeResolver};

/// Run one `cache` verb against `resolver`, writing results to `out`.
///
/// # Errors
///
/// A [`kernel::Error`] for an unrecognised artifact name (a refusal) or a
/// resolution failure that is not a recoverable miss.
pub fn run(
    action: &CacheAction,
    resolver: &TreeResolver<'_>,
    out: &mut dyn Write,
) -> Result<(), kernel::Error> {
    match action {
        CacheAction::Verify { name } => verify(resolver, name.as_deref(), out),
        CacheAction::Repair { name, force } => {
            repair(resolver, name.as_deref(), *force, out)
        }
        CacheAction::Ensure { names } => ensure(resolver, names, out),
        CacheAction::Prune { older_than } => prune(resolver, *older_than, out),
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
    // The caller holds the lease itself, so `ensure` prints the lease path
    // alongside the resolved tree path.
    for name in names {
        let sealed = resolver.materialise(name)?;
        let _ = writeln!(
            out,
            "{name}\t{}\t{}",
            sealed.path.display(),
            sealed.lease_path.display()
        );
    }
    Ok(())
}

fn prune(
    resolver: &TreeResolver<'_>,
    _older_than: Option<u64>,
    out: &mut dyn Write,
) -> Result<(), kernel::Error> {
    let mut reclaimed = 0;
    for artifact in pins::artifact_names() {
        reclaimed += resolver.prune(artifact)?.entries;
    }
    let _ = writeln!(out, "reclaimed {reclaimed} entries");
    Ok(())
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
