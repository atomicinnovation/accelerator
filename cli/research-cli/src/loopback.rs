//! The test-only seam: loopback source APIs, a stand-in address for a web
//! page's host, and a clock that logs its waits instead of sleeping, so the
//! integration suites run the real binary against a mock in real time.

use std::cell::Cell;
use std::io::Write as _;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use network::Host;
use research::sources::fetch::Clock;
use research::sources::request::Endpoint;
use research::sources::web::fetch::HostResolver;
use research::sources::web::fetch::PageRequest;
use research::sources::web::fetch::PageResponse;
use research::sources::web::fetch::PageTransport;
use research::sources::web::fetch::ResolveFailure;

use crate::web_command::WebAdapters;

const OPENALEX_API_URL: &str = "ACCELERATOR_OPENALEX_API_URL";
const ARXIV_API_URL: &str = "ACCELERATOR_ARXIV_API_URL";
const ARXIV_OAI_URL: &str = "ACCELERATOR_ARXIV_OAI_URL";
const CLOCK_LOG: &str = "ACCELERATOR_RESEARCH_TEST_CLOCK_LOG";
const CLOCK_EPOCH: &str = "ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH";
const CALL_BUDGET: &str = "ACCELERATOR_RESEARCH_TEST_CALL_BUDGET_MS";
const RESOLVE: &str = "ACCELERATOR_RESEARCH_TEST_RESOLVE";
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

/// A public address the stand-in name resolves to, so admission judges the
/// mock's page exactly as it would a real one; only the transport knows it
/// means the mock. It must stay a real public address: a documentation range
/// such as `192.0.2.0/24` is classified reserved and would be rejected before
/// the transport sees it.
pub const STAND_IN: IpAddr = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));

/// One page host answered by a loopback mock, named as `<name>=<socket>`.
#[derive(Debug, PartialEq, Eq)]
pub struct StandIn {
    name: String,
    loopback: SocketAddr,
}

/// The stand-in, when the variable names one.
///
/// # Errors
///
/// The rejected value when its socket is not a loopback one, so it can never
/// aim a request at a real host.
pub fn stand_in() -> Result<Option<StandIn>, String> {
    std::env::var(RESOLVE)
        .ok()
        .map(|raw| stand_in_from(&raw))
        .transpose()
}

fn stand_in_from(raw: &str) -> Result<StandIn, String> {
    raw.split_once('=')
        .and_then(|(name, socket)| {
            Some((name, socket.parse::<SocketAddr>().ok()?))
        })
        .filter(|(_, socket)| socket.ip().is_loopback())
        .map(|(name, loopback)| StandIn {
            name: name.to_ascii_lowercase(),
            loopback,
        })
        .ok_or_else(|| {
            format!(
                "E_BAD_TEST_RESOLVE: {RESOLVE}={raw:?} is not \
                 <name>=<loopback socket>"
            )
        })
}

/// The production adapters with the stand-in's name resolved to
/// [`STAND_IN`] and every request pinned there sent to the mock instead.
pub fn through_stand_in(
    stand_in: &StandIn,
    production: WebAdapters,
) -> WebAdapters {
    WebAdapters {
        resolver: Box::new(StandInResolver {
            name: stand_in.name.clone(),
            inner: production.resolver,
        }),
        transport: Box::new(StandInTransport {
            loopback: stand_in.loopback,
            inner: production.transport,
        }),
    }
}

struct StandInResolver {
    name: String,
    inner: Box<dyn HostResolver>,
}

impl HostResolver for StandInResolver {
    fn resolve(
        &self,
        host: &Host,
        port: u16,
        within: Duration,
    ) -> Result<Vec<SocketAddr>, ResolveFailure> {
        if host.to_string() == self.name {
            Ok(vec![SocketAddr::new(STAND_IN, port)])
        } else {
            self.inner.resolve(host, port, within)
        }
    }
}

struct StandInTransport {
    loopback: SocketAddr,
    inner: Box<dyn PageTransport>,
}

impl PageTransport for StandInTransport {
    fn get(&self, request: &PageRequest<'_>) -> PageResponse {
        let addresses = request
            .addresses
            .iter()
            .map(|address| {
                if address.ip() == STAND_IN {
                    self.loopback
                } else {
                    *address
                }
            })
            .collect::<Vec<_>>();
        self.inner.get(&PageRequest {
            addresses: &addresses,
            ..*request
        })
    }
}

/// A shorter call budget, when the variable names one in milliseconds, so a
/// suite can exhaust the deadline in real time.
pub fn call_budget() -> Option<Duration> {
    std::env::var(CALL_BUDGET)
        .ok()
        .and_then(|millis| millis.parse().ok())
        .map(Duration::from_millis)
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

const GUARD_PANIC: &str = "ACCELERATOR_RESEARCH_TEST_GUARD_PANIC";

/// Fails the guard once it has identified a confined call, so the suite can
/// prove a guard failure blocks.
pub fn panic_if_asked() {
    assert!(
        std::env::var_os(GUARD_PANIC).is_none(),
        "{GUARD_PANIC} is set"
    );
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::RefCell;
    use std::net::SocketAddr;
    use std::rc::Rc;
    use std::time::Duration;

    use network::host_reach::classify_address;
    use network::Host;
    use network::HostReach;
    use research::sources::web::fetch::HostResolver;
    use research::sources::web::fetch::PageRequest;
    use research::sources::web::fetch::PageResponse;
    use research::sources::web::fetch::PageTransport;
    use research::sources::web::fetch::ResolveFailure;
    use research::sources::web::page_url::PageUrl;

    use super::stand_in_from;
    use super::through_stand_in;
    use super::STAND_IN;
    use crate::web_command::WebAdapters;

    const MOCK: &str = "127.0.0.1:4321";

    fn socket(raw: &str) -> SocketAddr {
        raw.parse().expect("a socket")
    }

    #[derive(Clone, Default)]
    struct Recorded(Rc<RefCell<Vec<Vec<SocketAddr>>>>);

    struct RecordingResolver(Recorded);

    impl HostResolver for RecordingResolver {
        fn resolve(
            &self,
            _host: &Host,
            port: u16,
            _within: Duration,
        ) -> Result<Vec<SocketAddr>, ResolveFailure> {
            let answer = vec![SocketAddr::new([198, 51, 100, 7].into(), port)];
            self.0 .0.borrow_mut().push(answer.clone());
            Ok(answer)
        }
    }

    struct RecordingTransport(Recorded);

    impl PageTransport for RecordingTransport {
        fn get(&self, request: &PageRequest<'_>) -> PageResponse {
            self.0 .0.borrow_mut().push(request.addresses.to_vec());
            PageResponse::Refused
        }
    }

    fn adapters(resolved: &Recorded, sent: &Recorded) -> WebAdapters {
        through_stand_in(
            &stand_in_from(&format!("fixture.accelerator.test={MOCK}"))
                .expect("a stand-in"),
            WebAdapters {
                resolver: Box::new(RecordingResolver(resolved.clone())),
                transport: Box::new(RecordingTransport(sent.clone())),
            },
        )
    }

    fn host(raw: &str) -> Host {
        Host::canonicalise(raw).expect("a host")
    }

    #[test]
    fn the_stand_in_address_is_one_admission_admits() {
        assert_eq!(classify_address(STAND_IN), HostReach::Public);
    }

    #[test]
    fn only_a_name_and_loopback_socket_is_a_stand_in() {
        assert!(
            stand_in_from(&format!("fixture.accelerator.test={MOCK}")).is_ok()
        );
        assert!(stand_in_from("fixture.accelerator.test=[::1]:4321").is_ok());
        for refused in [
            "fixture.accelerator.test=93.184.216.34:80",
            "fixture.accelerator.test=10.0.0.1:80",
            "fixture.accelerator.test",
            "fixture.accelerator.test=127.0.0.1",
        ] {
            assert!(stand_in_from(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn the_stand_in_resolver_answers_only_its_own_name() {
        let resolved = Recorded::default();
        let adapters = adapters(&resolved, &Recorded::default());

        let own = adapters
            .resolver
            .resolve(&host("Fixture.Accelerator.Test"), 80, Duration::ZERO)
            .expect("an answer");
        assert_eq!(own, [SocketAddr::new(STAND_IN, 80)]);
        assert!(resolved.0.borrow().is_empty());

        adapters
            .resolver
            .resolve(&host("other.example"), 80, Duration::ZERO)
            .expect("an answer");
        assert_eq!(resolved.0.borrow().len(), 1);
    }

    #[test]
    fn the_stand_in_transport_sends_only_the_stand_in_address_to_the_mock() {
        let sent = Recorded::default();
        let adapters = adapters(&Recorded::default(), &sent);
        let url = PageUrl::parse("http://fixture.accelerator.test/")
            .expect("a page URL");
        let other = socket("198.51.100.7:80");

        adapters.transport.get(&PageRequest {
            url: &url,
            addresses: &[SocketAddr::new(STAND_IN, 80), other],
            accept: "text/html",
            max_body: 1,
            timeout: Duration::ZERO,
        });

        assert_eq!(*sent.0.borrow(), [vec![socket(MOCK), other]]);
    }
}
