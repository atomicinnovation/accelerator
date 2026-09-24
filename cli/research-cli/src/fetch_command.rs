//! One `research fetch` call: resolve the OpenAlex key when the call needs
//! one, then fetch within the call's deadline.
//!
//! Every read of the environment, the filesystem and the clock goes through
//! [`FetchPorts`], so the whole call runs against fakes.

use std::cell::Cell;
use std::rc::Rc;

use config::credentials::resolve_token;
use config::credentials::CredentialError;
use config::credentials::TokenKeys;
use config::credentials::TokenSource;
use config::Key;
use config_adapters::credentials::project_credential_context;
use config_adapters::credentials::CredentialPorts;
use research::classify::Response;
use research::fetch::fetch_arxiv;
use research::fetch::fetch_openalex;
use research::fetch::ArxivDecoder;
use research::fetch::ArxivFetch;
use research::fetch::ArxivPorts;
use research::fetch::Clock;
use research::fetch::ConfirmationCache;
use research::fetch::FetchOutcome;
use research::fetch::OpenAlexDecoder;
use research::fetch::OpenAlexFetch;
use research::fetch::OpenAlexPorts;
use research::fetch::PacingGate;
use research::fetch::Transport;
use research::request::ApiKey;
use research::request::ArxivRequest;
use research::request::Endpoint;
use research::request::Family;
use research::request::KeySource;
use research::request::OpenAlexRequest;
use research::request::UpstreamRequest;
use research::request::Verb;
use research::schedule::Deadline;

use crate::context::ProjectContext;

pub struct FetchPorts {
    pub clock: Rc<dyn Clock>,
    pub credentials: CredentialPorts,
}

/// A validated request paired with the adapters of the one source it asks,
/// so no call builds, or reads configuration for, a source it never asks.
pub enum SourceCall {
    OpenAlex(OpenAlexRequest, OpenAlexAdapters),
    Arxiv(ArxivRequest, ArxivAdapters),
}

pub struct OpenAlexAdapters {
    pub api: Endpoint,
    pub transport: Box<dyn Transport>,
    pub decoder: Box<dyn OpenAlexDecoder>,
    pub gate: Box<dyn PacingGate>,
}

pub struct ArxivAdapters {
    pub api: Endpoint,
    pub oai: Endpoint,
    pub transport: Box<dyn Transport>,
    pub decoder: Box<dyn ArxivDecoder>,
    pub gate: Box<dyn PacingGate>,
    pub confirmations: Box<dyn ConfirmationCache>,
}

/// A call that reached its source, however the source answered.
#[derive(Debug)]
pub struct Fetched {
    pub family: Family,
    pub verb: Verb,
    pub authenticated: bool,
    pub attempts: usize,
    pub outcome: FetchOutcome,
}

/// # Errors
///
/// A [`CredentialError`] when an OpenAlex key was configured but could not be
/// resolved, so the call is refused before any request.
pub fn run(
    ports: &FetchPorts,
    project: &ProjectContext,
    deadline: &Deadline,
    call: &SourceCall,
) -> Result<Fetched, CredentialError> {
    match call {
        SourceCall::OpenAlex(request, adapters) => {
            let key = resolve_key(ports, project, deadline)?;
            Ok(from_openalex(
                ports,
                adapters,
                deadline,
                request,
                key.as_ref(),
            ))
        }
        SourceCall::Arxiv(request, adapters) => {
            Ok(from_arxiv(ports, adapters, deadline, request))
        }
    }
}

fn from_openalex(
    ports: &FetchPorts,
    adapters: &OpenAlexAdapters,
    deadline: &Deadline,
    request: &OpenAlexRequest,
    key: Option<&ApiKey>,
) -> Fetched {
    let transport = Counted::around(adapters.transport.as_ref());
    let outcome = fetch_openalex(
        &OpenAlexFetch {
            request,
            api: &adapters.api,
            key,
        },
        &OpenAlexPorts {
            transport: &transport,
            decoder: adapters.decoder.as_ref(),
            clock: ports.clock.as_ref(),
            gate: adapters.gate.as_ref(),
        },
        deadline,
    );
    Fetched {
        family: Family::OpenAlex,
        verb: request.verb(),
        authenticated: key.is_some(),
        attempts: transport.sent.get(),
        outcome,
    }
}

fn from_arxiv(
    ports: &FetchPorts,
    adapters: &ArxivAdapters,
    deadline: &Deadline,
    request: &ArxivRequest,
) -> Fetched {
    let transport = Counted::around(adapters.transport.as_ref());
    let outcome = fetch_arxiv(
        &ArxivFetch {
            request,
            api: &adapters.api,
            oai: &adapters.oai,
        },
        &ArxivPorts {
            transport: &transport,
            decoder: adapters.decoder.as_ref(),
            clock: ports.clock.as_ref(),
            gate: adapters.gate.as_ref(),
            confirmations: adapters.confirmations.as_ref(),
        },
        deadline,
    );
    Fetched {
        family: Family::Arxiv,
        verb: request.verb(),
        authenticated: false,
        attempts: transport.sent.get(),
        outcome,
    }
}

/// The key command runs within whatever the deadline has left, not the
/// trackers' fixed timeout, so a slow helper cannot overrun the call.
fn resolve_key(
    ports: &FetchPorts,
    project: &ProjectContext,
    deadline: &Deadline,
) -> Result<Option<ApiKey>, CredentialError> {
    let context = project_credential_context(
        &project.root,
        &ports.credentials,
        project.config.as_ref(),
        deadline.remaining(ports.clock.now()),
    );
    match resolve_token(&context, &openalex_keys()) {
        Ok(resolved) => Ok(Some(ApiKey::new(
            resolved.value.expose().to_owned(),
            key_source(resolved.source),
        ))),
        Err(CredentialError::NoToken { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}
const ENV_KEY: &str = "ACCELERATOR_OPENALEX_API_KEY";
const ENV_KEY_COMMAND: &str = "ACCELERATOR_OPENALEX_API_KEY_CMD";

fn openalex_keys() -> TokenKeys {
    TokenKeys {
        env: ENV_KEY,
        env_command: ENV_KEY_COMMAND,
        value: catalogued("openalex.api_key"),
        command: catalogued("openalex.api_key_cmd"),
    }
}

#[allow(clippy::expect_used)]
fn catalogued(key: &str) -> Key {
    Key::parse(key).expect("a catalogued key parses")
}

fn key_source(source: TokenSource) -> KeySource {
    KeySource::new(match source {
        TokenSource::Env => ENV_KEY,
        TokenSource::EnvCommand => ENV_KEY_COMMAND,
        TokenSource::Personal => "openalex.api_key in config.local.md",
        TokenSource::PersonalCommand => {
            "openalex.api_key_cmd in config.local.md"
        }
        TokenSource::Shared => "openalex.api_key in config.md",
    })
}

/// Counts what reaches the wire, for the attempt count a failed or
/// unavailable call reports.
struct Counted<'a> {
    inner: &'a dyn Transport,
    sent: Cell<usize>,
}

impl<'a> Counted<'a> {
    const fn around(inner: &'a dyn Transport) -> Self {
        Self {
            inner,
            sent: Cell::new(0),
        }
    }
}

impl Transport for Counted<'_> {
    fn send(&self, request: &UpstreamRequest) -> Response {
        self.sent.set(self.sent.get() + 1);
        self.inner.send(request)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::path::Path;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::time::Duration;
    use std::time::Instant;
    use std::time::SystemTime;

    use config::credentials::CommandPolicy;
    use config::credentials::CredentialError;
    use config::credentials::Environment;
    use config::credentials::FileFacts;
    use config::credentials::FileState;
    use config::credentials::Provenance;
    use config::credentials::TokenCommandFailure;
    use config::credentials::TokenCommandRunner;
    use config::ConfigError;
    use config::Key;
    use config::Level;
    use config::Resolved;
    use config::Scalar;
    use config::Value;
    use config_adapters::credentials::CredentialPorts;
    use research::classify::Reason;
    use research::classify::Received;
    use research::classify::Response;
    use research::fetch::Attempted;
    use research::fetch::Clock;
    use research::fetch::ConfirmationCache;
    use research::fetch::FetchOutcome;
    use research::fetch::PacingGate;
    use research::fetch::Transport;
    use research::fetch::Unavailable;
    use research::request::ArxivId;
    use research::request::Endpoint;
    use research::request::FetchRequest;
    use research::request::UpstreamRequest;
    use research::schedule::Deadline;
    use research_adapters::arxiv_xml::XmlArxivDecoder;
    use research_adapters::openalex_json::JsonOpenAlexDecoder;
    use research_adapters::pacing::NoPacing;

    use super::run;
    use super::ArxivAdapters;
    use super::FetchPorts;
    use super::Fetched;
    use super::OpenAlexAdapters;
    use super::SourceCall;
    use crate::context::ProjectContext;

    const ROOT: &str = "/project";
    const PERSONAL: &str = "/project/.accelerator/config.local.md";

    const fn secs(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    struct VirtualClock {
        origin: Instant,
        elapsed: Cell<Duration>,
        slept: RefCell<Vec<Duration>>,
    }

    impl VirtualClock {
        fn new() -> Rc<Self> {
            Rc::new(Self {
                origin: Instant::now(),
                elapsed: Cell::new(Duration::ZERO),
                slept: RefCell::new(Vec::new()),
            })
        }

        fn advance(&self, by: Duration) {
            self.elapsed.set(self.elapsed.get() + by);
        }
    }

    impl Clock for VirtualClock {
        fn now(&self) -> Instant {
            self.origin + self.elapsed.get()
        }

        fn wall_now(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH + self.elapsed.get()
        }

        fn sleep(&self, duration: Duration) {
            self.slept.borrow_mut().push(duration);
            self.advance(duration);
        }
    }

    struct FixedEnvironment(Vec<(String, String)>);

    impl Environment for FixedEnvironment {
        fn read(&self, name: &str) -> Option<String> {
            self.0
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    struct UntrackedPersonalFile;

    impl FileFacts for UntrackedPersonalFile {
        fn inspect(&self, path: &Path) -> Result<FileState, String> {
            Ok(if path == Path::new(PERSONAL) {
                FileState::File { mode: 0o600 }
            } else {
                FileState::Absent
            })
        }
    }

    impl Provenance for UntrackedPersonalFile {
        fn is_tracked(&self, _path: &Path) -> bool {
            false
        }
    }

    /// A key command that takes `takes` of virtual time, or times out at
    /// its policy's limit when that is shorter.
    struct SlowKeyCommand {
        clock: Rc<VirtualClock>,
        takes: Duration,
        timeouts: Rc<RefCell<Vec<Duration>>>,
    }

    impl TokenCommandRunner for SlowKeyCommand {
        fn run(
            &self,
            _command: &str,
            policy: &CommandPolicy,
        ) -> Result<String, TokenCommandFailure> {
            self.timeouts.borrow_mut().push(policy.timeout);
            if self.takes > policy.timeout {
                self.clock.advance(policy.timeout);
                return Err(TokenCommandFailure::TimedOut);
            }
            self.clock.advance(self.takes);
            Ok("command-key".to_owned())
        }
    }

    struct PersonalKeyCommand;

    impl config::ConfigAccess for PersonalKeyCommand {
        fn get(
            &self,
            key: &Key,
            level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            let found = level == Some(Level::Personal)
                && key.to_string() == "openalex.api_key_cmd";
            Ok(if found {
                Resolved::Found(Value::Scalar(Scalar::String(
                    "fetch-key".to_owned(),
                )))
            } else {
                Resolved::Absent
            })
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("a fetch never writes config")
        }
    }

    /// Answers every request with a `5xx` at once, recording each bearer.
    struct AlwaysUnavailable {
        bearers: Rc<RefCell<Vec<Option<String>>>>,
    }

    impl Transport for AlwaysUnavailable {
        fn send(&self, request: &UpstreamRequest) -> Response {
            self.bearers
                .borrow_mut()
                .push(request.bearer().map(str::to_owned));
            Response::Received(Received::status(503))
        }
    }

    fn arxiv_fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../research-adapters/tests/fixtures/arxiv")
            .join(name);
        std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    }

    /// Answers the query with the recorded withdrawn entry and the
    /// confirmation with its `arXivRaw` record.
    struct RecordedWithdrawal;

    impl Transport for RecordedWithdrawal {
        fn send(&self, request: &UpstreamRequest) -> Response {
            let fixture = if request.url().contains("/oai?") {
                "oai-2608.21129.xml"
            } else {
                "lookup-2608.21129.xml"
            };
            Response::Received(Received {
                body: arxiv_fixture(fixture),
                ..Received::status(200)
            })
        }
    }

    struct CountingGate(Rc<Cell<usize>>);

    impl PacingGate for CountingGate {
        fn paced(
            &self,
            attempt: &mut dyn FnMut() -> Attempted,
            _deadline: &Deadline,
        ) -> Result<(), Unavailable> {
            self.0.set(self.0.get() + 1);
            attempt();
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct RememberingCache(Rc<RefCell<Vec<(ArxivId, bool)>>>);

    impl ConfirmationCache for RememberingCache {
        fn recall(&self, entry: &ArxivId) -> Option<bool> {
            self.0
                .borrow()
                .iter()
                .find(|(known, _)| known == entry)
                .map(|(_, withdrawn)| *withdrawn)
        }

        fn record(&self, entry: &ArxivId, withdrawn: bool) {
            self.0.borrow_mut().push((entry.clone(), withdrawn));
        }
    }

    struct Call {
        request: FetchRequest,
        gate_passes: Rc<Cell<usize>>,
        confirmations: RememberingCache,
        clock: Rc<VirtualClock>,
        deadline: Deadline,
        environment: Vec<(String, String)>,
        key_command_takes: Duration,
        timeouts: Rc<RefCell<Vec<Duration>>>,
        bearers: Rc<RefCell<Vec<Option<String>>>>,
    }

    impl Call {
        fn new() -> Self {
            let clock = VirtualClock::new();
            let deadline = Deadline::starting(clock.now(), secs(100), secs(30));
            Self {
                request: FetchRequest::parse(
                    "openalex",
                    "search",
                    &["graphs".to_owned()],
                    None,
                )
                .expect("a valid request"),
                gate_passes: Rc::default(),
                confirmations: RememberingCache::default(),
                clock,
                deadline,
                environment: Vec::new(),
                key_command_takes: Duration::ZERO,
                timeouts: Rc::default(),
                bearers: Rc::default(),
            }
        }

        fn arxiv_lookup(mut self, id: &str) -> Self {
            self.request =
                FetchRequest::parse("arxiv", "lookup", &[id.to_owned()], None)
                    .expect("a valid request");
            self
        }

        fn key_command_taking(mut self, takes: Duration) -> Self {
            self.key_command_takes = takes;
            self
        }

        fn env(mut self, name: &str, value: &str) -> Self {
            self.environment.push((name.to_owned(), value.to_owned()));
            self
        }

        fn run(&self) -> Result<Fetched, CredentialError> {
            let ports = FetchPorts {
                clock: self.clock.clone(),
                credentials: CredentialPorts {
                    environment: Box::new(FixedEnvironment(
                        self.environment.clone(),
                    )),
                    files: Box::new(UntrackedPersonalFile),
                    commands: Box::new(SlowKeyCommand {
                        clock: self.clock.clone(),
                        takes: self.key_command_takes,
                        timeouts: self.timeouts.clone(),
                    }),
                    provenance: Box::new(UntrackedPersonalFile),
                },
            };
            let project = ProjectContext {
                root: PathBuf::from(ROOT),
                config: Box::new(PersonalKeyCommand),
            };
            run(&ports, &project, &self.deadline, &self.source_call())
        }

        fn source_call(&self) -> SourceCall {
            match self.request.clone() {
                FetchRequest::OpenAlex(request) => SourceCall::OpenAlex(
                    request,
                    OpenAlexAdapters {
                        api: Endpoint::openalex(),
                        transport: Box::new(AlwaysUnavailable {
                            bearers: self.bearers.clone(),
                        }),
                        decoder: Box::new(JsonOpenAlexDecoder),
                        gate: Box::new(NoPacing),
                    },
                ),
                FetchRequest::Arxiv(request) => SourceCall::Arxiv(
                    request,
                    ArxivAdapters {
                        api: Endpoint::arxiv_api(),
                        oai: Endpoint::arxiv_oai(),
                        transport: Box::new(RecordedWithdrawal),
                        decoder: Box::new(XmlArxivDecoder),
                        gate: Box::new(CountingGate(self.gate_passes.clone())),
                        confirmations: Box::new(self.confirmations.clone()),
                    },
                ),
            }
        }

        fn slept(&self) -> Vec<Duration> {
            self.clock.slept.borrow().clone()
        }
    }

    fn unavailable(fetched: &Fetched) -> Unavailable {
        match fetched.outcome {
            FetchOutcome::Unavailable(unavailable) => unavailable,
            ref other => panic!("expected unavailable, got {other:?}"),
        }
    }

    #[test]
    fn a_slow_key_command_leaves_room_for_only_the_attempts_that_fit() {
        let call = Call::new().key_command_taking(secs(65));

        let fetched = call.run().expect("a fetch");

        assert_eq!(fetched.attempts, 2);
        assert_eq!(call.slept(), [secs(3)]);
        assert_eq!(unavailable(&fetched).reason(), Reason::UpstreamError);
        assert!(fetched.authenticated);
    }

    #[test]
    fn an_attempt_that_would_end_exactly_at_the_deadline_is_admitted() {
        let fetched = Call::new()
            .key_command_taking(secs(70))
            .run()
            .expect("a fetch");

        assert_eq!(fetched.attempts, 1);
    }

    #[test]
    fn a_key_command_that_leaves_no_room_for_an_attempt_is_rate_limited() {
        let fetched = Call::new()
            .key_command_taking(secs(75))
            .run()
            .expect("a fetch");

        assert_eq!(fetched.attempts, 0);
        assert_eq!(unavailable(&fetched).reason(), Reason::RateLimited);
    }

    #[test]
    fn the_key_command_is_bounded_by_the_budget_the_deadline_has_left() {
        let call = Call::new().key_command_taking(secs(500));
        call.clock.advance(secs(12));

        let refusal = call.run().expect_err("a refusal");

        assert!(matches!(refusal, CredentialError::TokenCmdTimedOut { .. }));
        assert_eq!(*call.timeouts.borrow(), [secs(88)]);
        assert!(call.bearers.borrow().is_empty());
    }

    #[test]
    fn an_environment_key_wins_without_running_the_key_command() {
        let call = Call::new().env("ACCELERATOR_OPENALEX_API_KEY", "env-key");

        call.run().expect("a fetch");

        assert!(call.timeouts.borrow().is_empty());
        assert_eq!(
            call.bearers.borrow().first(),
            Some(&Some("env-key".to_owned()))
        );
    }

    #[test]
    fn an_arxiv_lookup_confirms_its_withdrawal_through_the_gate_once() {
        let call = Call::new().arxiv_lookup("2608.21129");

        let fetched = call.run().expect("a fetch");

        let FetchOutcome::Records(records) = &fetched.outcome else {
            panic!("expected records, got {:?}", fetched.outcome);
        };
        assert_eq!(records.len(), 1);
        assert!(records[0].withdrawn());
        assert_eq!(fetched.attempts, 2);
        assert_eq!(call.gate_passes.get(), 2);
        assert_eq!(
            *call.confirmations.0.borrow(),
            [(ArxivId::parse("2608.21129v2").expect("an ID"), true)]
        );
        assert!(!fetched.authenticated);
        assert!(call.timeouts.borrow().is_empty());
    }
}
