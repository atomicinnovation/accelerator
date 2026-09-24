//! The test-only seam: a loopback OpenAlex API and a clock that logs its
//! waits instead of sleeping, so the integration suites run the real binary
//! against a mock in real time.

use std::cell::Cell;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::fetch::Clock;
use research::request::Endpoint;

const API_URL: &str = "ACCELERATOR_OPENALEX_API_URL";
const CLOCK_LOG: &str = "ACCELERATOR_RESEARCH_TEST_CLOCK_LOG";
const LOOPBACK_ORIGINS: [&str; 3] =
    ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"];

/// The overriding OpenAlex API, if one is set.
///
/// # Errors
///
/// The rejected value when it is not a loopback URL, so the override can
/// never aim a key at a real host.
pub fn openalex_api() -> Result<Option<Endpoint>, String> {
    let Some(raw) = std::env::var(API_URL).ok() else {
        return Ok(None);
    };
    if LOOPBACK_ORIGINS
        .iter()
        .any(|origin| raw.starts_with(origin))
    {
        Ok(Some(Endpoint::new(&raw)))
    } else {
        Err(format!(
            "E_BAD_API_URL: {API_URL}={raw:?} is not a loopback URL"
        ))
    }
}

pub fn logging_clock() -> Option<LoggingClock> {
    std::env::var_os(CLOCK_LOG).map(|log| LoggingClock {
        log: PathBuf::from(log),
        waited: Cell::new(Duration::ZERO),
    })
}

/// Real time plus every wait it was asked for, each appended to the log in
/// milliseconds rather than slept.
pub struct LoggingClock {
    log: PathBuf,
    waited: Cell<Duration>,
}

impl Clock for LoggingClock {
    fn now(&self) -> Instant {
        Instant::now() + self.waited.get()
    }

    fn wall_now(&self) -> SystemTime {
        SystemTime::now() + self.waited.get()
    }

    fn sleep(&self, duration: Duration) {
        self.waited.set(self.waited.get() + duration);
        let appended = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)
            .and_then(|mut log| writeln!(log, "{}", duration.as_millis()));
        if let Err(error) = appended {
            eprintln!(
                "could not log a wait to {}: {error}",
                self.log.display()
            );
        }
    }
}
