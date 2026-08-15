//! The lockhash namespace digest, against the shipped lockfile and against
//! `ensure-playwright.sh`'s own function.
//!
//! `ensure-playwright.sh` is the only thing that populates the namespace; the
//! executor only ever reads it. Off by anything and every invocation returns
//! `playwright-not-installed` at exit 3 on a machine whose runtime is installed
//! perfectly well.
//!
//! Two assertions, because they fail for different reasons. The golden catches
//! a change in this crate's arithmetic. The cross-check catches the two
//! implementations diverging — and is written to fail loudly rather than skip
//! if the tooling it needs is absent, since a silently-skipped cross-check is
//! the same as not having one.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use design_adapters::paths::lockhash;
use design_adapters::paths::lockhash_of;

type TestError = Box<dyn std::error::Error>;

/// The digest of the lockfile as shipped today. Regenerate deliberately when
/// the Playwright dependency set changes — which is exactly when the namespace
/// is *meant* to move, and when `ensure-playwright.sh` will populate the new
/// one.
const SHIPPED_LOCKHASH: &str = "ef1f88a3";

fn plugin_root() -> PathBuf {
    // The crate sits at cli/design-adapters, so the plugin root is two up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn shipped_lockfile() -> PathBuf {
    plugin_root()
        .join("skills/design/inventory-design/scripts/playwright")
        .join("package-lock.json")
}

#[test]
fn the_shipped_lockfile_hashes_to_its_recorded_namespace(
) -> Result<(), TestError> {
    let lockfile = shipped_lockfile();
    assert!(
        lockfile.is_file(),
        "{} must exist: the namespace is derived from it",
        lockfile.display()
    );
    assert_eq!(
        lockhash(&lockfile)?,
        SHIPPED_LOCKHASH,
        "the namespace digest moved; ensure-playwright.sh populates the old one"
    );
    Ok(())
}

/// The same digest as `sha256sum FILE | cut -c1-8`, computed by the host's own
/// tooling rather than by this crate.
///
/// `ensure-playwright.sh` picks `sha256sum` when present and falls back to
/// `shasum -a 256`, so both are tried here for the same reason.
#[test]
fn the_digest_agrees_with_the_bootstrap_script_s_own_function(
) -> Result<(), TestError> {
    let lockfile = shipped_lockfile();

    let mut attempted = Vec::new();
    for (program, arguments) in
        [("sha256sum", &[][..]), ("shasum", &["-a", "256"])]
    {
        attempted.push(program);
        let Ok(output) = Command::new(program)
            .args(arguments)
            .arg(&lockfile)
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8(output.stdout)?;
        let digest: String = stdout.chars().take(8).collect();
        assert_eq!(
            lockhash(&lockfile)?,
            digest,
            "{program} disagrees with the port's digest"
        );
        return Ok(());
    }

    Err(format!(
        "neither of {attempted:?} is available, so the cross-check against \
         ensure-playwright.sh's own function cannot run — failing rather than \
         skipping, since a skipped cross-check is the same as none"
    )
    .into())
}

/// A namespace is a directory name, so it must survive being used as one.
#[test]
fn the_digest_is_a_usable_path_component() -> Result<(), TestError> {
    let digest = lockhash(&shipped_lockfile())?;
    assert!(!digest.contains('/'));
    assert!(!digest.contains(std::path::MAIN_SEPARATOR));
    assert_eq!(Path::new(&digest).components().count(), 1);
    Ok(())
}

/// Same bytes, same answer, regardless of where they were read from — so a
/// copied plugin tree resolves the same namespace as the original.
#[test]
fn the_digest_depends_only_on_the_bytes() -> Result<(), TestError> {
    let bytes = std::fs::read(shipped_lockfile())?;
    let work = tempfile::tempdir()?;
    let elsewhere = work.path().join("package-lock.json");
    std::fs::write(&elsewhere, &bytes)?;

    assert_eq!(lockhash(&elsewhere)?, lockhash_of(&bytes));
    assert_eq!(lockhash(&elsewhere)?, SHIPPED_LOCKHASH);
    Ok(())
}

#[test]
fn an_unreadable_lockfile_names_the_path_it_could_not_read(
) -> Result<(), TestError> {
    let Err(error) = lockhash(Path::new("/nonexistent-by-construction.json"))
    else {
        return Err("expected a failure".into());
    };
    let message = error.to_string();
    assert!(message.contains("nonexistent-by-construction.json"));
    assert!(message.contains("Playwright namespace"));
    Ok(())
}
