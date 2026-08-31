//! The never-fail boundary over the `vcs::VcsReporter` port.
//!
//! `status`/`log` must return text on any failure (matching the shell's original
//! `2>/dev/null || echo`), so this folds an adapter `Err` and a cleanly-unwinding
//! panic to the `(status|log unavailable)` fallback, warn-logging the
//! failing adapter's `gix`/`jj-lib` token on an `adapter =` field so the failure
//! is diagnosable via `ACCELERATOR_LOG` rather than reading like a clean repo.
//!
//! The `catch_unwind` fold depends on `panic = "unwind"`; `cli/Cargo.toml`'s
//! `[profile.release]` leaves the default, guarded by a manifest test. It does
//! not cover a panic inside a destructor during unwinding (Rust aborts on a
//! double panic), nor a wall-clock hang — a thread cannot be safely interrupted
//! in-process. `discover`/`kind` stay outside the guard: they are the in-process
//! facts path `detect`/`guard` already rely on unguarded.

use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use tracing::warn;
use vcs::RepoRoot as _;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs::VcsReporter;
use vcs_adapters::library::InProcessProbe;

/// The failing adapter's backend token. Deliberately distinct from the
/// library adapter's own `vcs = "git"/"jj"` warnings, so a log consumer knows
/// the status/log fallback is keyed on `adapter`, not `vcs`.
const fn adapter_token(kind: VcsKind) -> &'static str {
    match kind {
        VcsKind::Jj => "jj-lib",
        VcsKind::Git | VcsKind::None => "gix",
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic")
}

/// Reads and renders one subcommand's report, never failing. `subject` names the
/// subcommand for the diagnostic; `fallback` is its `(… unavailable)` literal.
pub fn run<T>(
    start: &Path,
    reporter: &dyn VcsReporter,
    read: impl FnOnce(&dyn VcsReporter, &Path, VcsKind) -> Result<T, kernel::Error>,
    format: impl FnOnce(&T) -> String,
    subject: &str,
    fallback: &str,
) -> String {
    let probe = InProcessProbe;
    let root = probe.discover(start);
    let kind = root
        .as_deref()
        .map_or(VcsKind::Git, |root| probe.kind(root));
    let dir = root.as_deref().unwrap_or(start);

    let outcome =
        panic::catch_unwind(AssertUnwindSafe(|| read(reporter, dir, kind)));
    match outcome {
        Ok(Ok(report)) => format(&report),
        Ok(Err(error)) => {
            warn!(
                adapter = adapter_token(kind),
                %error,
                "could not render {subject}"
            );
            fallback.to_owned()
        }
        Err(payload) => {
            warn!(
                adapter = adapter_token(kind),
                panic = panic_message(&*payload),
                "panicked rendering {subject}"
            );
            fallback.to_owned()
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;

    use vcs::VcsKind;
    use vcs::VcsReporter;

    use crate::log;
    use crate::status;

    struct FailingReporter;

    impl VcsReporter for FailingReporter {
        fn status_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<vcs::status::StatusReport, kernel::Error> {
            Err(kernel::Error::Failed("adapter refused".to_owned()))
        }

        fn log_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<vcs::log::LogReport, kernel::Error> {
            Err(kernel::Error::Failed("adapter refused".to_owned()))
        }
    }

    struct PanickingReporter;

    impl VcsReporter for PanickingReporter {
        fn status_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<vcs::status::StatusReport, kernel::Error> {
            panic!("adapter exploded")
        }

        fn log_report(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<vcs::log::LogReport, kernel::Error> {
            panic!("adapter exploded")
        }
    }

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl tracing_subscriber::fmt::MakeWriter<'_> for Buffer {
        type Writer = Self;

        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture(run: impl FnOnce()) -> String {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, run);
        let bytes = buffer.0.lock().unwrap().clone();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn marked(marker: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(marker)).expect("marker dir");
        dir
    }

    #[test]
    fn a_failing_git_status_folds_to_the_fallback_and_warns_gix() {
        let dir = marked(".git");
        let mut output = String::new();
        let logs = capture(|| {
            output = status::run(dir.path(), &FailingReporter);
        });
        assert_eq!(output, "(status unavailable)");
        assert!(logs.contains("gix"), "expected the gix token: {logs}");
        assert!(
            !logs.contains("jj-lib"),
            "the jj token must not leak: {logs}"
        );
    }

    #[test]
    fn a_failing_jj_status_folds_to_the_fallback_and_warns_jj_lib() {
        let dir = marked(".jj");
        let mut output = String::new();
        let logs = capture(|| {
            output = status::run(dir.path(), &FailingReporter);
        });
        assert_eq!(output, "(status unavailable)");
        assert!(logs.contains("jj-lib"), "expected the jj-lib token: {logs}");
    }

    #[test]
    fn a_failing_git_log_folds_to_the_fallback_and_warns_gix() {
        let dir = marked(".git");
        let mut output = String::new();
        let logs = capture(|| {
            output = log::run(dir.path(), &FailingReporter);
        });
        assert_eq!(output, "(log unavailable)");
        assert!(logs.contains("gix"), "expected the gix token: {logs}");
    }

    #[test]
    fn a_failing_jj_log_folds_to_the_fallback_and_warns_jj_lib() {
        let dir = marked(".jj");
        let mut output = String::new();
        let logs = capture(|| {
            output = log::run(dir.path(), &FailingReporter);
        });
        assert_eq!(output, "(log unavailable)");
        assert!(logs.contains("jj-lib"), "expected the jj-lib token: {logs}");
    }

    #[test]
    fn a_panicking_status_reporter_folds_to_the_fallback_and_warns() {
        let dir = marked(".git");
        let mut output = String::new();
        let logs = capture(|| {
            output = status::run(dir.path(), &PanickingReporter);
        });
        assert_eq!(output, "(status unavailable)");
        assert!(logs.contains("gix"), "expected the gix token: {logs}");
        assert!(
            logs.contains("adapter exploded"),
            "the panic message must reach the diagnostic: {logs}"
        );
    }

    #[test]
    fn a_panicking_log_reporter_folds_to_the_fallback_and_warns() {
        let dir = marked(".jj");
        let mut output = String::new();
        let logs = capture(|| {
            output = log::run(dir.path(), &PanickingReporter);
        });
        assert_eq!(output, "(log unavailable)");
        assert!(logs.contains("jj-lib"), "expected the jj-lib token: {logs}");
    }

    #[test]
    fn the_release_profile_does_not_disable_unwinding() {
        // catch_unwind is a no-op under panic = "abort", so the never-fail
        // panic fold would silently stop protecting the shipped binary. Forcing
        // a real gix/jj-lib panic through the release binary is impractical, so
        // this pins the manifest instead.
        let manifest = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"),
        )
        .expect("read cli/Cargo.toml");
        let release = manifest
            .split("[profile.release]")
            .nth(1)
            .expect("a [profile.release] section")
            .split("\n[")
            .next()
            .expect("the section body");
        assert!(
            !release.contains("panic") || !release.contains("abort"),
            "[profile.release] must not set panic = \"abort\": {release}"
        );
    }
}
