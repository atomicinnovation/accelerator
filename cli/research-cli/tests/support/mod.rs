//! Runs `accelerator-research` as a subprocess against a mock, through the
//! `test-loopback` API URL seam and the non-sleeping clock. Only compiled into
//! tests gated on that feature.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;

use http_test_support::MockHTTPServer;
use research::sources::service::Service;
use vcs_test_support::hermetic::Hermetic;

pub const SELECT: &str = "select=id,doi,display_name,type,authorships,\
                          primary_location,is_retracted,\
                          abstract_inverted_index,publication_date";

/// Milliseconds since the Unix epoch at which every test clock's wall time
/// starts, so values one process persists compare deterministically with
/// another's.
pub const CLOCK_EPOCH_MS: &str = "1800000000000";

pub const CALL_BUDGET_MS: &str = "ACCELERATOR_RESEARCH_TEST_CALL_BUDGET_MS";

/// The stored arguments of a search, as a ticket record holds them.
pub fn search_binding(query: &str, limit: u8) -> String {
    format!("\"verb\":\"search\",\"query\":{query:?},\"limit\":{limit}")
}

/// The stored arguments of a DOI metadata call, as a ticket record holds
/// them.
pub fn doi_binding(doi: &str) -> String {
    format!("\"verb\":\"metadata\",\"doi\":{doi:?}")
}

/// A ticket record seeded relative to the test clock's epoch, which is
/// every call's wall time when it joins the queue.
pub struct SeededTicket<'a> {
    pub binding: &'a str,
    pub issued_ago: u64,
    pub presented_ago: u64,
    pub ended_ago: Option<u64>,
    pub last_retryable: Option<&'a str>,
}

impl SeededTicket<'_> {
    /// A ticket whose last call ended `ended_ago` seconds before the epoch.
    pub fn absent(
        binding: &str,
        issued_ago: u64,
        ended_ago: u64,
    ) -> SeededTicket<'_> {
        SeededTicket {
            binding,
            issued_ago,
            presented_ago: issued_ago.min(ended_ago + 1),
            ended_ago: Some(ended_ago),
            last_retryable: None,
        }
    }

    /// A ticket whose call is still running, unless its lock is free.
    pub const fn live(binding: &str, issued_ago: u64) -> SeededTicket<'_> {
        SeededTicket {
            binding,
            issued_ago,
            presented_ago: issued_ago,
            ended_ago: None,
            last_retryable: None,
        }
    }
}

fn now_ms() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    u64::try_from(now).expect("milliseconds fit")
}

fn epoch_ms() -> u64 {
    CLOCK_EPOCH_MS.parse().expect("an epoch in milliseconds")
}

pub fn openalex_fixture(name: &str) -> String {
    adapter_fixture("openalex", name)
}

pub fn arxiv_fixture(name: &str) -> String {
    adapter_fixture("arxiv", name)
}

pub fn web_fixture(name: &str) -> String {
    adapter_fixture("web", name)
}

pub fn crossref_fixture(name: &str) -> String {
    adapter_fixture("crossref", name)
}

fn adapter_fixture(family: &str, name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../research-adapters/tests/fixtures")
        .join(family)
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// A page holding exactly the named recorded work fixtures.
pub fn page_of(names: &[&str]) -> String {
    let works = names
        .iter()
        .map(|name| openalex_fixture(name))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"meta\":{{}},\"results\":[{works}]}}")
}

/// A file or directory of a service's queue and pacing state.
#[derive(Debug, Clone, Copy)]
pub enum StateFile {
    ServingLock,
    Pacing,
    RequestLog,
    ContentionLog,
    Queue,
    QueueLock,
}

impl StateFile {
    const fn name(self) -> &'static str {
        match self {
            Self::ServingLock => "serving-0.lock",
            Self::Pacing => "pacing",
            Self::RequestLog => "requests.log",
            Self::ContentionLog => "contention.log",
            Self::Queue => "queue",
            Self::QueueLock => "queue/queue.lock",
        }
    }
}

/// A file or directory the previous release kept arXiv's state in.
#[derive(Debug, Clone, Copy)]
pub enum LegacyFile {
    Queue,
    Pacing,
    Lock,
}

impl LegacyFile {
    const fn name(self) -> &'static str {
        match self {
            Self::Queue => "arxiv-queue",
            Self::Pacing => "arxiv-pacing",
            Self::Lock => "arxiv.lock",
        }
    }
}

/// One live ticket per serving slot at the front of a service's queue,
/// held until dropped, so every call joining behind it waits.
pub struct Barrier {
    _tickets: Vec<File>,
}

/// A scratch project: a repository directory with an `.accelerator/`
/// directory, optionally under git so files can be tracked.
pub struct Project {
    work: tempfile::TempDir,
    root: PathBuf,
    git: Option<Hermetic>,
}

impl Project {
    pub fn new() -> Self {
        let work = tempfile::Builder::new()
            .prefix("research-cli-")
            .tempdir()
            .expect("tempdir");
        let root = work.path().join("repo");
        std::fs::create_dir_all(root.join(".accelerator"))
            .expect("mkdir .accelerator");
        Self {
            work,
            root,
            git: None,
        }
    }

    pub fn under_git() -> Self {
        let mut project = Self::new();
        let env = Hermetic::rooted_at(project.work.path()).expect("hermetic");
        env.git(&["init", "--quiet"], &project.root)
            .expect("git init");
        project.git = Some(env);
        project
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn shared_config(self, body: &str) -> Self {
        std::fs::write(self.root.join(".accelerator/config.md"), body)
            .expect("write config.md");
        self
    }

    pub fn personal_config(self, body: &str, mode: u32) -> Self {
        let path = self.root.join(".accelerator/config.local.md");
        std::fs::write(&path, body).expect("write config.local.md");
        set_mode(&path, mode);
        self
    }

    pub fn track_personal_config(self) -> Self {
        let env = self.git.as_ref().expect("a git project");
        env.git(&["add", ".accelerator/config.local.md"], &self.root)
            .expect("git add");
        self
    }

    fn clock_log(&self) -> PathBuf {
        self.work.path().join("clock.log")
    }

    /// Forgets the waits earlier calls logged, so the next call's are its
    /// own.
    pub fn clear_clock_log(&self) {
        let _ = std::fs::remove_file(self.clock_log());
    }

    /// The research scratch directory under the default `paths.tmp`.
    pub fn research_scratch(&self) -> PathBuf {
        self.root.join(".accelerator/tmp/research")
    }

    pub fn service_state(&self, service: Service, file: StateFile) -> PathBuf {
        self.research_scratch()
            .join("services")
            .join(service.code())
            .join(file.name())
    }

    pub fn arxiv_state(&self, file: StateFile) -> PathBuf {
        self.service_state(Service::Arxiv, file)
    }

    pub fn legacy_arxiv_state(&self, file: LegacyFile) -> PathBuf {
        self.research_scratch().join(file.name())
    }

    /// Copies the previous release's captured arXiv state into this
    /// project, and returns the ticket it printed.
    pub fn seed_legacy_fixture(&self) -> String {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/legacy-arxiv-queue");
        let queue = self.legacy_arxiv_state(LegacyFile::Queue);
        std::fs::create_dir_all(&queue).expect("mkdir the legacy queue");
        for entry in std::fs::read_dir(fixture.join(LegacyFile::Queue.name()))
            .expect("list the fixture queue")
        {
            let entry = entry.expect("an entry");
            std::fs::copy(entry.path(), queue.join(entry.file_name()))
                .expect("copy a ticket record");
        }
        std::fs::copy(
            fixture.join(LegacyFile::Pacing.name()),
            self.legacy_arxiv_state(LegacyFile::Pacing),
        )
        .expect("copy the legacy pacing");
        std::fs::read_to_string(fixture.join("ticket.txt"))
            .expect("the fixture's ticket")
            .trim()
            .to_owned()
    }

    /// Holds a lock the previous release kept, as one of its calls would.
    pub fn hold_legacy(&self, file: LegacyFile) -> File {
        Self::hold_path(&self.legacy_arxiv_state(file))
    }

    /// Fills every serving slot of `service` with a live ticket issued
    /// just now, ahead of any call the test then starts.
    pub fn hold_barrier(&self, service: Service) -> Barrier {
        let queue = self.service_queue_dir(service);
        let tickets = (1..=service.pacing().slots().get())
            .map(|slot| {
                let ticket = format!("{slot}-{slot:06x}");
                Self::seed_ticket_into(
                    &queue,
                    &ticket,
                    &SeededTicket::live(&search_binding("barrier", 10), 0),
                    now_ms(),
                );
                Self::hold_path(&queue.join(format!("{ticket}.lock")))
            })
            .collect();
        Barrier { _tickets: tickets }
    }

    pub fn queue_dir(&self) -> PathBuf {
        self.service_queue_dir(Service::Arxiv)
    }

    fn service_queue_dir(&self, service: Service) -> PathBuf {
        let queue = self.service_state(service, StateFile::Queue);
        std::fs::create_dir_all(&queue).expect("mkdir the queue");
        queue
    }

    pub fn seed_ticket(&self, ticket: &str, seeded: &SeededTicket<'_>) {
        self.seed_ticket_before(ticket, seeded, epoch_ms());
    }

    /// Seeds a ticket into `service`'s queue relative to the test clock's
    /// epoch.
    pub fn seed_service_ticket(
        &self,
        service: Service,
        ticket: &str,
        seeded: &SeededTicket<'_>,
    ) {
        Self::seed_ticket_into(
            &self.service_queue_dir(service),
            ticket,
            seeded,
            epoch_ms(),
        );
    }

    /// Holds a ticket in `service`'s queue, as the call presenting it
    /// would.
    pub fn hold_service_ticket(&self, service: Service, ticket: &str) -> File {
        Self::hold_path(
            &self
                .service_queue_dir(service)
                .join(format!("{ticket}.lock")),
        )
    }

    /// Records a `Retry-After` in `service`'s pacing that every caller must
    /// honour until `seconds` after the test clock's epoch.
    pub fn defer_service(&self, service: Service, seconds: u64) {
        let path = self.service_state(service, StateFile::Pacing);
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("mkdir service state");
        std::fs::write(
            path,
            format!(
                "{{\"schema_version\":1,\"last_sent_ms\":null,\
                 \"next_free_ms\":null,\"last_finish_ms\":null,\
                 \"not_before_ms\":{}}}",
                epoch_ms() + seconds * 1000
            ),
        )
        .expect("write pacing");
    }

    /// The send stamps `service`'s gate logged.
    pub fn service_requests(&self, service: Service) -> Vec<u64> {
        std::fs::read_to_string(
            self.service_state(service, StateFile::RequestLog),
        )
        .unwrap_or_default()
        .lines()
        .map(|line| line.parse().expect("a stamp in milliseconds"))
        .collect()
    }

    /// Seeds a ticket for calls on the real clock, relative to now.
    pub fn seed_ticket_now(&self, ticket: &str, seeded: &SeededTicket<'_>) {
        self.seed_ticket_before(ticket, seeded, now_ms());
    }

    fn seed_ticket_before(
        &self,
        ticket: &str,
        seeded: &SeededTicket<'_>,
        now_ms: u64,
    ) {
        Self::seed_ticket_into(&self.queue_dir(), ticket, seeded, now_ms);
    }

    fn seed_ticket_into(
        queue: &Path,
        ticket: &str,
        seeded: &SeededTicket<'_>,
        now_ms: u64,
    ) {
        let at = |ago: u64| now_ms - ago * 1000;
        let ended = seeded
            .ended_ago
            .map_or_else(|| "null".to_owned(), |ago| at(ago).to_string());
        let last_retryable = seeded
            .last_retryable
            .map_or_else(|| "null".to_owned(), |code| format!("{code:?}"));
        std::fs::write(
            queue.join(format!("{ticket}.json")),
            format!(
                "{{\"schema_version\":1,\"issued_ms\":{},\
                 \"presented_ms\":{},\"ended_ms\":{ended},\
                 \"last_retryable\":{last_retryable},{}}}",
                at(seeded.issued_ago),
                at(seeded.presented_ago),
                seeded.binding,
            ),
        )
        .expect("seed a ticket");
    }

    /// Holds `file` exclusively, as another call would.
    pub fn hold(&self, file: StateFile) -> File {
        Self::hold_path(&self.arxiv_state(file))
    }

    fn hold_path(path: &Path) -> File {
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("mkdir");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .expect("open to hold");
        rustix::fs::flock(
            &file,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .expect("hold the lock");
        file
    }

    pub fn hold_ticket(&self, ticket: &str) -> File {
        Self::hold_path(&self.queue_dir().join(format!("{ticket}.lock")))
    }

    /// Whether a call holds the ticket, read with a shared probe as the
    /// queue reads it.
    pub fn ticket_is_held(&self, ticket: &str) -> bool {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.queue_dir().join(format!("{ticket}.lock")))
            .is_ok_and(|lock| {
                rustix::fs::flock(
                    &lock,
                    rustix::fs::FlockOperation::NonBlockingLockShared,
                )
                .is_err()
            })
    }

    /// The tickets with a record in the queue, in issue order.
    pub fn tickets(&self) -> Vec<String> {
        self.service_tickets(Service::Arxiv)
    }

    /// The tickets with a record in `service`'s queue, in issue order.
    pub fn service_tickets(&self, service: Service) -> Vec<String> {
        let mut tickets: Vec<(u64, String)> =
            std::fs::read_dir(self.service_queue_dir(service))
                .expect("list the queue")
                .filter_map(|entry| {
                    let name =
                        entry.ok()?.file_name().to_string_lossy().into_owned();
                    let ticket = name.strip_suffix(".json")?.to_owned();
                    let number = ticket.split_once('-')?.0.parse().ok()?;
                    Some((number, ticket))
                })
                .collect();
        tickets.sort();
        tickets.into_iter().map(|(_, ticket)| ticket).collect()
    }

    pub fn ticket_exists(&self, ticket: &str) -> bool {
        self.queue_dir().join(format!("{ticket}.json")).exists()
    }

    pub fn queue_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(self.queue_dir())
            .expect("list the queue")
            .map(|entry| {
                let entry = entry.expect("an entry");
                let bytes = std::fs::read(entry.path()).unwrap_or_default();
                (entry.file_name().to_string_lossy().into_owned(), bytes)
            })
            .collect();
        files.sort();
        files
    }

    pub fn contention_lines(&self) -> Vec<String> {
        std::fs::read_to_string(self.arxiv_state(StateFile::ContentionLog))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .expect("chmod");
}

#[cfg(not(unix))]
const fn set_mode(_path: &Path, _mode: u32) {}

pub fn config_with(section: &str) -> String {
    format!("---\nopenalex:\n{section}---\n")
}

pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub waits: Vec<u64>,
}

impl Run {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|error| {
            panic!("stdout is not JSON ({error}): {}", self.stdout)
        })
    }
}

/// Runs `fetch <args>` in `project` against `server` on the logging test
/// clock, with every OpenAlex key variable cleared unless `env` sets it.
pub fn fetch(
    project: &Project,
    server: &MockHTTPServer,
    args: &[&str],
    env: &[(&str, &str)],
) -> Run {
    let mut command = command(project, server, args);
    command
        .env("ACCELERATOR_RESEARCH_TEST_CLOCK_LOG", project.clock_log())
        .env("ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH", CLOCK_EPOCH_MS);
    for (name, value) in env {
        command.env(name, value);
    }
    let output = command.output().expect("run accelerator-research");
    let waits = std::fs::read_to_string(project.clock_log())
        .unwrap_or_default()
        .lines()
        .map(|line| line.parse().expect("a wait in milliseconds"))
        .collect();
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        waits,
    }
}

/// Runs `fetch <args>` in `dir` on the logging test clock, with no API URL
/// override or stand-in resolution unless `env` sets one.
pub fn fetch_from(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    let scratch = tempfile::tempdir().expect("tempdir");
    let clock_log = scratch.path().join("clock.log");
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-research"));
    command
        .arg("fetch")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_RESEARCH_TEST_CLOCK_LOG", &clock_log)
        .env("ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH", CLOCK_EPOCH_MS)
        .env_remove("ACCELERATOR_RESEARCH_TEST_RESOLVE")
        .env_remove(CALL_BUDGET_MS)
        .env_remove("ACCELERATOR_OPENALEX_API_URL")
        .env_remove("ACCELERATOR_ARXIV_API_URL")
        .env_remove("ACCELERATOR_ARXIV_OAI_URL")
        .env_remove("ACCELERATOR_CROSSREF_API_URL")
        .stdin(Stdio::null());
    for (name, value) in env {
        command.env(name, value);
    }
    let output = command.output().expect("run accelerator-research");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        waits: Vec::new(),
    }
}

/// Starts `fetch <args>` in `project` against `server` on the real clock,
/// so its waits really pass.
pub fn spawn_in_real_time(
    project: &Project,
    server: &MockHTTPServer,
    args: &[&str],
) -> Child {
    spawn_in_real_time_with(project, server, args, &[])
}

pub fn spawn_in_real_time_with(
    project: &Project,
    server: &MockHTTPServer,
    args: &[&str],
    env: &[(&str, &str)],
) -> Child {
    let mut command = command(project, server, args);
    for (name, value) in env {
        command.env(name, value);
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn accelerator-research")
}

fn command(
    project: &Project,
    server: &MockHTTPServer,
    args: &[&str],
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-research"));
    command
        .arg("fetch")
        .args(args)
        .current_dir(project.root())
        .env("ACCELERATOR_OPENALEX_API_URL", server.base_url())
        .env("ACCELERATOR_ARXIV_API_URL", server.base_url())
        .env("ACCELERATOR_ARXIV_OAI_URL", server.base_url())
        .env("ACCELERATOR_CROSSREF_API_URL", server.base_url())
        .env_remove("ACCELERATOR_RESEARCH_TEST_CLOCK_LOG")
        .env_remove("ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH")
        .env_remove("ACCELERATOR_OPENALEX_API_KEY")
        .env_remove("ACCELERATOR_OPENALEX_API_KEY_CMD")
        .stdin(Stdio::null());
    command
}

/// Asserts `actual` against the committed golden `name` under
/// `tests/fixtures/goldens/`; `UPDATE_GOLDEN=1` rewrites it instead.
pub fn assert_golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/goldens")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("golden parent"))
            .expect("mkdir goldens");
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .expect("missing golden — run with UPDATE_GOLDEN=1 to create it");
    assert_eq!(
        actual,
        expected,
        "stdout diverged from golden {}; re-run with UPDATE_GOLDEN=1 to accept",
        path.display()
    );
}
