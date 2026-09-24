//! One `research fetch` call: resolve the OpenAlex key, then fetch within the
//! call's deadline.
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
use research::fetch::fetch_openalex;
use research::fetch::Clock;
use research::fetch::FetchOutcome;
use research::fetch::OpenAlexDecoder;
use research::fetch::OpenAlexFetch;
use research::fetch::OpenAlexPorts;
use research::fetch::PacingGate;
use research::fetch::Transport;
use research::request::ApiKey;
use research::request::Endpoint;
use research::request::Family;
use research::request::FetchRequest;
use research::request::KeySource;
use research::request::UpstreamRequest;
use research::request::Verb;
use research::schedule::Deadline;

use crate::context::ProjectContext;

pub struct FetchPorts {
    pub clock: Rc<dyn Clock>,
    pub credentials: CredentialPorts,
    pub openalex: OpenAlexAdapters,
}

pub struct OpenAlexAdapters {
    pub api: Endpoint,
    pub transport: Box<dyn Transport>,
    pub decoder: Box<dyn OpenAlexDecoder>,
    pub gate: Box<dyn PacingGate>,
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

#[derive(Debug)]
pub enum Refusal {
    Credential(CredentialError),
    FamilyUnavailable(Family),
}

/// # Errors
///
/// A [`Refusal`] when the call cannot be made at all: the key could not be
/// resolved, or the family has no adapters yet.
pub fn run(
    ports: &FetchPorts,
    project: &ProjectContext,
    deadline: &Deadline,
    request: &FetchRequest,
) -> Result<Fetched, Refusal> {
    let FetchRequest::OpenAlex(request) = request else {
        return Err(Refusal::FamilyUnavailable(request.family()));
    };
    let key = resolve_key(ports, project, deadline)?;
    let transport = Counted::around(ports.openalex.transport.as_ref());
    let outcome = fetch_openalex(
        &OpenAlexFetch {
            request,
            api: &ports.openalex.api,
            key: key.as_ref(),
        },
        &OpenAlexPorts {
            transport: &transport,
            decoder: ports.openalex.decoder.as_ref(),
            clock: ports.clock.as_ref(),
            gate: ports.openalex.gate.as_ref(),
        },
        deadline,
    );
    Ok(Fetched {
        family: Family::OpenAlex,
        verb: request.verb(),
        authenticated: key.is_some(),
        attempts: transport.sent.get(),
        outcome,
    })
}

/// The key command runs within whatever the deadline has left, not the
/// trackers' fixed timeout, so a slow helper cannot overrun the call.
fn resolve_key(
    ports: &FetchPorts,
    project: &ProjectContext,
    deadline: &Deadline,
) -> Result<Option<ApiKey>, Refusal> {
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
        Err(error) => Err(Refusal::Credential(error)),
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
    use research::fetch::Clock;
    use research::fetch::FetchOutcome;
    use research::fetch::Transport;
    use research::fetch::Unavailable;
    use research::request::Endpoint;
    use research::request::FetchRequest;
    use research::request::UpstreamRequest;
    use research::schedule::Deadline;
    use research_adapters::openalex_json::JsonOpenAlexDecoder;
    use research_adapters::pacing::NoPacing;

    use super::run;
    use super::FetchPorts;
    use super::Fetched;
    use super::OpenAlexAdapters;
    use super::Refusal;
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

    struct Call {
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
                clock,
                deadline,
                environment: Vec::new(),
                key_command_takes: Duration::ZERO,
                timeouts: Rc::default(),
                bearers: Rc::default(),
            }
        }

        fn key_command_taking(mut self, takes: Duration) -> Self {
            self.key_command_takes = takes;
            self
        }

        fn env(mut self, name: &str, value: &str) -> Self {
            self.environment.push((name.to_owned(), value.to_owned()));
            self
        }

        fn run(&self) -> Result<Fetched, Refusal> {
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
                openalex: OpenAlexAdapters {
                    api: Endpoint::openalex(),
                    transport: Box::new(AlwaysUnavailable {
                        bearers: self.bearers.clone(),
                    }),
                    decoder: Box::new(JsonOpenAlexDecoder),
                    gate: Box::new(NoPacing),
                },
            };
            let project = ProjectContext {
                root: PathBuf::from(ROOT),
                config: Box::new(PersonalKeyCommand),
            };
            let request = FetchRequest::parse(
                "openalex",
                "search",
                &["graphs".to_owned()],
                None,
            )
            .expect("a valid request");
            run(&ports, &project, &self.deadline, &request)
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

        assert!(matches!(
            refusal,
            Refusal::Credential(CredentialError::TokenCmdTimedOut { .. })
        ));
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
}
