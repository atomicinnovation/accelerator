use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BLOCKING_BOUND: Duration = Duration::from_secs(10);

#[derive(Clone, Default)]
pub struct Buffer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log buffer").extend_from_slice(bytes);
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

pub fn logged(act: impl FnOnce()) -> String {
    let buffer = Buffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, act);
    let bytes = buffer.0.lock().expect("log buffer").clone();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[allow(clippy::panic)]
pub fn within<T: Send + 'static>(
    bound: Duration,
    what: &str,
    run: impl FnOnce() -> T + Send + 'static,
) -> T {
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _ = sender.send(tracing::dispatcher::with_default(&dispatch, run));
    });
    match receiver.recv_timeout(bound) {
        Ok(outcome) => outcome,
        Err(RecvTimeoutError::Timeout) => panic!("{what} blocked"),
        Err(RecvTimeoutError::Disconnected) => match worker.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => panic!("{what} ended without an outcome"),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{logged, within};

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "the probe blocked")]
    fn a_run_that_outlasts_its_bound_fails_instead_of_hanging() {
        use super::super::lease::take_single_flight;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lock");
        let _held = take_single_flight(&path).expect("first holder");

        within(Duration::from_millis(100), "the probe", move || {
            take_single_flight(&path).map(drop)
        })
        .expect("taken");
    }

    #[test]
    fn a_warning_raised_within_a_bounded_run_reaches_the_callers_capture() {
        let log = logged(|| {
            within(Duration::from_secs(10), "the warning", || {
                tracing::warn!("x");
            });
        });

        assert!(log.contains("WARN"), "no warning captured: {log}");
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn a_panic_within_a_bounded_run_is_reraised_not_reported_as_blocked() {
        within::<()>(Duration::from_secs(10), "the run", || panic!("boom"));
    }
}
