//! A fail-closed boundary over the tracking questions: gix and jj-lib may
//! panic, and an asking process must survive with an answer that refuses
//! trust.
//!
//! The fold depends on `panic = "unwind"`, which `cli/Cargo.toml`'s
//! `[profile.release]` keeps. It does not cover a hang: like the `git` and
//! `jj` reads it re-implements, a question blocks until its I/O completes.
//!
//! A panic hook runs before `catch_unwind` regains control, so the first fold
//! wraps whichever hook is installed and silences it for a folding thread
//! only. Every other panic still reaches the hook its composition root chose.

use std::any::type_name;
use std::any::Any;
use std::cell::Cell;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::Once;

use tracing::warn;

use vcs::tracking::FileTracking;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;

/// The text a panic carried, or `"panic"` for a payload that is not a string.
#[must_use]
pub fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic")
}

/// Answers as `T` does, except that a panic answers `Unknown` and an
/// incomplete empty set of roots.
pub struct PanicFold<T>(pub T);

impl<T: RepositoryTracking> RepositoryTracking for PanicFold<T> {
    fn file_tracking(&self, path: &Path) -> FileTracking {
        folded::<T, _>(
            "whether a file is tracked",
            FileTracking::Unknown,
            || self.0.file_tracking(path),
        )
    }

    fn repository_roots(&self, directory: &Path) -> RootsAnswer {
        let incomplete = RootsAnswer {
            roots: Vec::new(),
            complete: false,
        };
        folded::<T, _>(
            "which repositories enclose a directory",
            incomplete,
            || self.0.repository_roots(directory),
        )
    }
}

fn folded<T, A>(question: &str, refusal: A, ask: impl FnOnce() -> A) -> A {
    silence_folded_panics();
    let folding = Folding::begin();
    let answer = panic::catch_unwind(AssertUnwindSafe(ask));
    drop(folding);
    answer.unwrap_or_else(|payload| {
        warn!(
            adapter = type_name::<T>(),
            panic = panic_message(&*payload),
            "panicked answering {question}"
        );
        refusal
    })
}

thread_local! {
    static FOLDING: Cell<bool> = const { Cell::new(false) };
}

struct Folding {
    enclosing: bool,
}

impl Folding {
    fn begin() -> Self {
        Self {
            enclosing: FOLDING.replace(true),
        }
    }
}

impl Drop for Folding {
    fn drop(&mut self) {
        FOLDING.set(self.enclosing);
    }
}

fn silence_folded_panics() {
    static WRAPPED: Once = Once::new();
    WRAPPED.call_once(|| {
        let chosen = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            if !FOLDING.get() {
                chosen(panic);
            }
        }));
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;

    use vcs::tracking::FileTracking;
    use vcs::tracking::RepositoryTracking;
    use vcs::tracking::RootsAnswer;

    use super::panic_message;
    use super::PanicFold;

    struct Panicking;

    impl RepositoryTracking for Panicking {
        fn file_tracking(&self, _path: &Path) -> FileTracking {
            panic!("index exploded")
        }

        fn repository_roots(&self, _directory: &Path) -> RootsAnswer {
            panic!("workspace exploded")
        }
    }

    struct Answering;

    impl RepositoryTracking for Answering {
        fn file_tracking(&self, _path: &Path) -> FileTracking {
            FileTracking::Tracked
        }

        fn repository_roots(&self, directory: &Path) -> RootsAnswer {
            RootsAnswer {
                roots: vec![directory.to_path_buf()],
                complete: true,
            }
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

    fn logged<A>(ask: impl FnOnce() -> A) -> (A, String) {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buffer.clone())
            .with_ansi(false)
            .finish();
        let answer = tracing::subscriber::with_default(subscriber, ask);
        let bytes = buffer.0.lock().unwrap().clone();
        (answer, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[test]
    fn a_panic_asking_about_a_file_is_unknown_and_warned() {
        let (answer, logs) = logged(|| {
            PanicFold(Panicking).file_tracking(Path::new("/repo/file"))
        });

        assert_eq!(answer, FileTracking::Unknown);
        assert!(logs.contains("WARN"), "{logs}");
        assert!(logs.contains("index exploded"), "{logs}");
    }

    #[test]
    fn a_panic_asking_for_roots_is_an_incomplete_empty_answer_and_warned() {
        let (answer, logs) = logged(|| {
            PanicFold(Panicking).repository_roots(Path::new("/repo"))
        });

        assert_eq!(
            answer,
            RootsAnswer {
                roots: vec![],
                complete: false,
            }
        );
        assert!(logs.contains("WARN"), "{logs}");
        assert!(logs.contains("workspace exploded"), "{logs}");
    }

    #[test]
    fn an_answer_passes_through_unwarned() {
        let (answers, logs) = logged(|| {
            let fold = PanicFold(Answering);
            (
                fold.file_tracking(Path::new("/repo/file")),
                fold.repository_roots(Path::new("/repo")),
            )
        });

        assert_eq!(
            answers,
            (
                FileTracking::Tracked,
                RootsAnswer {
                    roots: vec![Path::new("/repo").to_path_buf()],
                    complete: true,
                }
            )
        );
        assert_eq!(logs, "");
    }

    #[test]
    fn a_panic_message_is_its_string_payload() {
        let owned: Box<dyn std::any::Any + Send> = Box::new(String::from("a"));
        let borrowed: Box<dyn std::any::Any + Send> = Box::new("b");
        let other: Box<dyn std::any::Any + Send> = Box::new(7_u8);

        assert_eq!(panic_message(&*owned), "a");
        assert_eq!(panic_message(&*borrowed), "b");
        assert_eq!(panic_message(&*other), "panic");
    }
}
