//! The test-only seam: loopback source APIs and a clock that logs its waits
//! instead of sleeping, so the integration suites run the real binary against
//! a mock in real time.

use std::cell::Cell;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::fetch::Clock;
use research::request::Endpoint;

const OPENALEX_API_URL: &str = "ACCELERATOR_OPENALEX_API_URL";
const ARXIV_API_URL: &str = "ACCELERATOR_ARXIV_API_URL";
const ARXIV_OAI_URL: &str = "ACCELERATOR_ARXIV_OAI_URL";
const CLOCK_LOG: &str = "ACCELERATOR_RESEARCH_TEST_CLOCK_LOG";
const CLOCK_EPOCH: &str = "ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH";
const LOOPBACK_ORIGINS: [&str; 3] =
    ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"];

/// The OpenAlex API, overridden when the variable is set.
///
/// # Errors
///
/// The rejected value when the override is not a loopback URL.
pub fn openalex_api() -> Result<Endpoint, String> {
    overridden(OPENALEX_API_URL, Endpoint::openalex)
}

/// The arXiv query API, overridden when the variable is set.
///
/// # Errors
///
/// The rejected value when the override is not a loopback URL.
pub fn arxiv_api() -> Result<Endpoint, String> {
    overridden(ARXIV_API_URL, Endpoint::arxiv_api)
}

/// The arXiv OAI-PMH endpoint, overridden when the variable is set.
///
/// # Errors
///
/// The rejected value when the override is not a loopback URL.
pub fn arxiv_oai() -> Result<Endpoint, String> {
    overridden(ARXIV_OAI_URL, Endpoint::arxiv_oai)
}

/// Only a loopback override is honoured, so it can never aim a key at a
/// real host.
fn overridden(
    variable: &str,
    production: fn() -> Endpoint,
) -> Result<Endpoint, String> {
    let Some(raw) = std::env::var(variable).ok() else {
        return Ok(production());
    };
    if LOOPBACK_ORIGINS
        .iter()
        .any(|origin| raw.starts_with(origin))
    {
        Ok(Endpoint::new(&raw))
    } else {
        Err(format!(
            "E_BAD_API_URL: {variable}={raw:?} is not a loopback URL"
        ))
    }
}

/// The logging clock, when a log is named. Its wall time starts at the
/// epoch variable's milliseconds when set, so processes sharing one epoch
/// compare the times they persist deterministically.
pub fn logging_clock() -> Option<LoggingClock> {
    let wall_epoch = std::env::var(CLOCK_EPOCH)
        .ok()
        .and_then(|millis| millis.parse().ok())
        .map(|millis| SystemTime::UNIX_EPOCH + Duration::from_millis(millis));
    std::env::var_os(CLOCK_LOG).map(|log| LoggingClock {
        log: PathBuf::from(log),
        wall_epoch,
        waited: Cell::new(Duration::ZERO),
    })
}

/// Real time plus every wait it was asked for, each appended to the log in
/// milliseconds rather than slept.
pub struct LoggingClock {
    log: PathBuf,
    wall_epoch: Option<SystemTime>,
    waited: Cell<Duration>,
}

impl Clock for LoggingClock {
    fn now(&self) -> Instant {
        Instant::now() + self.waited.get()
    }

    fn wall_now(&self) -> SystemTime {
        self.wall_epoch.unwrap_or_else(SystemTime::now) + self.waited.get()
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
