//! A virtual clock, captured diagnostics, and a scratch project directory
//! for the file-backed adapters' suites.

#![allow(dead_code, clippy::expect_used)]

use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::sources::fetch::Clock;
use research::sources::queue::Binding;
use research::sources::queue::Joining;
use research::sources::queue::Nonce;
use research::sources::queue::ServiceQueue as _;
use research::sources::queue::Ticket;
use research::sources::schedule::Deadline;
use research::sources::service::Service;
use research_adapters::contention::FileContentionLog;
use research_adapters::diagnostics::Diagnostics;
use research_adapters::pacing::FilePacingGate;
use research_adapters::queue::random_nonce;
use research_adapters::queue::FileServiceQueue;
use research_adapters::scratch::ScratchDir;
use research_adapters::services::AdapterContext;
use research_adapters::services::PreviousRelease;

pub const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

pub fn millis(at: SystemTime) -> u128 {
    at.duration_since(SystemTime::UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis()
}

/// Time that passes only when a test advances it or an adapter sleeps.
pub struct RecordingClock {
    origin: Instant,
    wall_origin: SystemTime,
    elapsed: Cell<Duration>,
    slept: RefCell<Vec<Duration>>,
}

impl RecordingClock {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            origin: Instant::now(),
            wall_origin: SystemTime::UNIX_EPOCH + secs(1_800_000_000),
            elapsed: Cell::new(Duration::ZERO),
            slept: RefCell::new(Vec::new()),
        })
    }

    pub fn ago(&self, by: Duration) -> SystemTime {
        self.wall_origin + self.elapsed.get() - by
    }

    pub fn advance(&self, by: Duration) {
        self.elapsed.set(self.elapsed.get() + by);
    }

    pub fn slept(&self) -> Vec<Duration> {
        self.slept.borrow().clone()
    }
}

impl Clock for RecordingClock {
    fn now(&self) -> Instant {
        self.origin + self.elapsed.get()
    }

    fn wall_now(&self) -> SystemTime {
        self.wall_origin + self.elapsed.get()
    }

    fn sleep(&self, duration: Duration) {
        self.slept.borrow_mut().push(duration);
        self.advance(duration);
    }
}

#[derive(Default)]
pub struct RecordedDiagnostics(RefCell<Vec<String>>);

impl RecordedDiagnostics {
    pub fn lines(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}

impl Diagnostics for RecordedDiagnostics {
    fn report(&self, line: &str) {
        self.0.borrow_mut().push(line.to_owned());
    }
}

/// A file or directory of a service's queue and pacing state.
#[derive(Debug, Clone, Copy)]
pub enum StateFile {
    /// The first serving slot's lock, the only one a one-slot service has.
    ServingLock,
    ServingSlot(u8),
    PacingLock,
    Pacing,
    RequestLog,
    ContentionLog,
    Queue,
}

impl StateFile {
    fn name(self) -> String {
        match self {
            Self::ServingLock => Self::ServingSlot(0).name(),
            Self::ServingSlot(slot) => format!("serving-{slot}.lock"),
            Self::PacingLock => "pacing.lock".to_owned(),
            Self::Pacing => "pacing".to_owned(),
            Self::RequestLog => "requests.log".to_owned(),
            Self::ContentionLog => "contention.log".to_owned(),
            Self::Queue => "queue".to_owned(),
        }
    }
}

/// The pacing state a service's gate persists between calls.
#[derive(Default)]
pub struct PacingRecord {
    pub last_sent: Option<SystemTime>,
    pub next_free: Option<SystemTime>,
    pub last_finish: Option<SystemTime>,
    pub not_before: Option<SystemTime>,
}

pub fn arxiv_gate(
    scratch: &Scratch,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> FilePacingGate {
    service_gate(Service::Arxiv, &scratch.dir(), clock, diagnostics)
}

pub fn service_gate(
    service: Service,
    research: &ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> FilePacingGate {
    FilePacingGate::new(service, research, clock, diagnostics, Vec::new())
}

/// A gate that also takes each of `companions` while it serves.
pub fn gate_with_companions(
    service: Service,
    research: &ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
    companions: Vec<PathBuf>,
) -> FilePacingGate {
    FilePacingGate::new(service, research, clock, diagnostics, companions)
}

/// What every service's adapters share, over `scratch`.
pub fn adapter_context(
    scratch: &Scratch,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> AdapterContext {
    AdapterContext {
        research: scratch.dir(),
        clock,
        diagnostics,
        nonces: Rc::new(random_nonce),
    }
}

/// The previous release's arXiv state, where it kept it.
pub fn previous_release(scratch: &Scratch) -> PreviousRelease {
    PreviousRelease {
        queue: scratch.dir().nested("arxiv-queue"),
        lock: scratch.legacy_arxiv_state(LegacyFile::Lock),
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

/// Reports nowhere, for a gate built inside another caller's sleep.
pub struct Silent;

impl Diagnostics for Silent {
    fn report(&self, _line: &str) {}
}

type Hook = Box<dyn FnOnce() + Send>;

/// Virtual time any thread can read and advance. A registered hook runs
/// inside the next sleep, before time advances, and another inside the next
/// wall-clock read, so a test can act at exactly those moments.
#[derive(Clone)]
pub struct HookedClock(Arc<HookedTime>);

struct HookedTime {
    origin: Instant,
    wall_origin: SystemTime,
    elapsed: Mutex<Duration>,
    slept: Mutex<Vec<Duration>>,
    on_sleep: Mutex<Option<Hook>>,
    on_wall_read: Mutex<Option<Hook>>,
}

impl HookedClock {
    pub fn new() -> Self {
        Self(Arc::new(HookedTime {
            origin: Instant::now(),
            wall_origin: SystemTime::UNIX_EPOCH + secs(1_800_000_000),
            elapsed: Mutex::default(),
            slept: Mutex::default(),
            on_sleep: Mutex::default(),
            on_wall_read: Mutex::default(),
        }))
    }

    pub fn on_next_sleep(&self, hook: impl FnOnce() + Send + 'static) {
        *self.0.on_sleep.lock().expect("hook") = Some(Box::new(hook));
    }

    pub fn on_next_wall_read(&self, hook: impl FnOnce() + Send + 'static) {
        *self.0.on_wall_read.lock().expect("hook") = Some(Box::new(hook));
    }

    /// The wall time, read without running any hook.
    pub fn current_wall(&self) -> SystemTime {
        self.0.wall_origin + *self.0.elapsed.lock().expect("elapsed")
    }

    pub fn advance(&self, by: Duration) {
        *self.0.elapsed.lock().expect("elapsed") += by;
    }

    pub fn slept(&self) -> Vec<Duration> {
        self.0.slept.lock().expect("slept").clone()
    }

    pub fn shared(&self) -> Rc<dyn Clock> {
        Rc::new(self.clone())
    }
}

impl Clock for HookedClock {
    fn now(&self) -> Instant {
        self.0.origin + *self.0.elapsed.lock().expect("elapsed")
    }

    fn wall_now(&self) -> SystemTime {
        let hook = self.0.on_wall_read.lock().expect("hook").take();
        if let Some(hook) = hook {
            hook();
        }
        self.current_wall()
    }

    fn sleep(&self, duration: Duration) {
        self.0.slept.lock().expect("slept").push(duration);
        let hook = self.0.on_sleep.lock().expect("hook").take();
        if let Some(hook) = hook {
            hook();
        }
        self.advance(duration);
    }
}

pub fn arxiv_contention_log(
    scratch: &Scratch,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> FileContentionLog {
    FileContentionLog::new(Service::Arxiv, &scratch.dir(), clock, diagnostics)
}

pub fn arxiv_queue(
    research: &ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
    nonces: Box<dyn Fn() -> Nonce>,
) -> FileServiceQueue {
    FileServiceQueue::new(
        Service::Arxiv,
        research,
        clock,
        diagnostics,
        nonces,
        Vec::new(),
    )
}

/// Joins `queue` as an arXiv call would.
pub fn join_queue<'q>(
    queue: &'q FileServiceQueue,
    binding: &Binding,
    presented: Option<&Ticket>,
    deadline: &Deadline,
) -> Joining<'q> {
    queue.join(binding, presented, deadline)
}

/// A project root holding the `research` scratch directory the adapters
/// keep their state in.
pub struct Scratch {
    root: tempfile::TempDir,
}

impl Scratch {
    pub fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("tempdir"),
        }
    }

    pub fn dir(&self) -> ScratchDir {
        ScratchDir::new(self.root.path(), &self.directory())
    }

    fn directory(&self) -> PathBuf {
        self.root.path().join("tmp/research")
    }

    pub fn service_state(&self, service: Service, file: StateFile) -> PathBuf {
        self.directory()
            .join("services")
            .join(service.code())
            .join(file.name())
    }

    pub fn arxiv_state(&self, file: StateFile) -> PathBuf {
        self.service_state(Service::Arxiv, file)
    }

    pub fn legacy_arxiv_state(&self, file: LegacyFile) -> PathBuf {
        self.directory().join(file.name())
    }

    /// Writes a ticket record into `service`'s queue, as a call of that
    /// service would have left it.
    pub fn seed_service_ticket(&self, service: Service, ticket: &str) {
        Self::seed_record_in(
            &self.service_state(service, StateFile::Queue),
            ticket,
        );
    }

    /// Writes a ticket record where the previous release kept its queue.
    pub fn seed_legacy_ticket(&self, ticket: &str) {
        Self::seed_record_in(
            &self.legacy_arxiv_state(LegacyFile::Queue),
            ticket,
        );
    }

    fn seed_record_in(directory: &Path, ticket: &str) {
        std::fs::create_dir_all(directory).expect("mkdir queue");
        std::fs::write(
            directory.join(format!("{ticket}.json")),
            format!(
                "{{\"schema_version\":1,\"issued_ms\":0,\"presented_ms\":0,\
                 \"ended_ms\":0,\"last_retryable\":null,{GRAPHS}}}"
            ),
        )
        .expect("seed a ticket");
    }

    pub fn queue_directory(&self) -> PathBuf {
        self.arxiv_state(StateFile::Queue)
    }

    pub fn write_pacing(&self, record: &PacingRecord) {
        self.write_service_pacing(Service::Arxiv, record);
    }

    pub fn write_service_pacing(
        &self,
        service: Service,
        record: &PacingRecord,
    ) {
        write_pacing_at(
            &self.service_state(service, StateFile::Pacing),
            record,
        );
    }

    pub fn stored_pacing(&self) -> serde_json::Value {
        self.stored_service_pacing(Service::Arxiv)
    }

    pub fn stored_service_pacing(&self, service: Service) -> serde_json::Value {
        serde_json::from_str(
            &self
                .read(self.service_state(service, StateFile::Pacing))
                .expect("a pacing state"),
        )
        .expect("JSON pacing state")
    }

    pub fn queue_path(&self, name: &str) -> PathBuf {
        let directory = self.queue_directory();
        std::fs::create_dir_all(&directory).expect("mkdir queue");
        directory.join(name)
    }

    /// Writes a ticket's record as another call would have, dated by its
    /// presentation.
    pub fn seed_ticket(&self, ticket: &str, record: &TicketRecord<'_>) {
        let ms = |at: SystemTime| millis(at).to_string();
        let ended = record.ended.map_or_else(|| "null".to_owned(), ms);
        let last_retryable = record
            .last_retryable
            .map_or_else(|| "null".to_owned(), |code| format!("\"{code}\""));
        let bytes = format!(
            "{{\"schema_version\":1,\"issued_ms\":{},\"presented_ms\":{},\
             \"ended_ms\":{ended},\"last_retryable\":{last_retryable},{}}}",
            ms(record.issued),
            ms(record.presented),
            record.binding,
        );
        self.seed_raw_record(
            &format!("{ticket}.json"),
            bytes.as_bytes(),
            record.presented,
        );
    }

    pub fn seed_raw_record(
        &self,
        name: &str,
        bytes: &[u8],
        modified: SystemTime,
    ) {
        let path = self.queue_path(name);
        std::fs::write(&path, bytes).expect("write record");
        set_modified(&path, modified);
    }

    pub fn seed_lock(&self, ticket: &str) {
        drop(self.open_queue_file(&format!("{ticket}.lock")));
    }

    pub fn seed_queue_lock(&self) {
        drop(self.open_queue_file("queue.lock"));
    }

    pub fn seed_temp_file(&self, name: &str, modified: SystemTime) {
        let path = self.queue_path(name);
        std::fs::write(&path, b"partial").expect("write temp file");
        set_modified(&path, modified);
    }

    pub fn open_queue_file(&self, name: &str) -> File {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.queue_path(name))
            .expect("open queue file")
    }

    /// Holds a ticket live, as another call's place would.
    pub fn hold_ticket(&self, ticket: &str) -> File {
        let lock = self.open_queue_file(&format!("{ticket}.lock"));
        rustix::fs::flock(
            &lock,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .expect("the test holds the ticket");
        lock
    }

    pub fn hold_queue_lock(&self) -> File {
        let lock = self.open_queue_file("queue.lock");
        rustix::fs::flock(
            &lock,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .expect("the test holds the queue lock");
        lock
    }

    pub fn queue_lock_is_free(&self) -> bool {
        let lock = self.open_queue_file("queue.lock");
        rustix::fs::flock(
            &lock,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .is_ok()
    }

    /// Whether some holder has the ticket's lock exclusively, read with a
    /// shared probe as the queue reads it.
    pub fn ticket_is_held(&self, ticket: &str) -> bool {
        let Ok(lock) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.queue_path(&format!("{ticket}.lock")))
        else {
            return false;
        };
        rustix::fs::flock(
            &lock,
            rustix::fs::FlockOperation::NonBlockingLockShared,
        )
        .is_err()
    }

    pub fn queue_names(&self) -> Vec<String> {
        let mut names = std::fs::read_dir(self.queue_directory())
            .map(|entries| {
                entries
                    .map(|entry| {
                        entry
                            .expect("an entry")
                            .file_name()
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// Every queue file's name and bytes, for asserting nothing changed.
    pub fn queue_files(&self) -> Vec<(String, Vec<u8>)> {
        self.queue_names()
            .into_iter()
            .map(|name| {
                let bytes =
                    std::fs::read(self.queue_path(&name)).unwrap_or_default();
                (name, bytes)
            })
            .collect()
    }

    pub fn record_bytes(&self, ticket: &str) -> Option<Vec<u8>> {
        std::fs::read(self.queue_path(&format!("{ticket}.json"))).ok()
    }

    pub fn root_path(&self) -> PathBuf {
        self.root.path().to_owned()
    }

    pub fn record(&self, ticket: &str) -> Option<serde_json::Value> {
        std::fs::read(self.queue_path(&format!("{ticket}.json")))
            .ok()
            .map(|bytes| serde_json::from_slice(&bytes).expect("a JSON record"))
    }

    /// `name` relative to the research directory, or an absolute path such
    /// as [`Self::arxiv_state`] gives, with its parent created.
    fn path(&self, name: impl AsRef<Path>) -> PathBuf {
        let path = self.directory().join(name);
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("mkdir scratch");
        path
    }

    pub fn write(&self, name: impl AsRef<Path>, contents: &str) {
        std::fs::write(self.path(name), contents).expect("write scratch file");
    }

    pub fn read(&self, name: impl AsRef<Path>) -> Option<String> {
        std::fs::read_to_string(self.path(name)).ok()
    }

    pub fn mkdir(&self, name: impl AsRef<Path>) {
        std::fs::create_dir_all(self.path(name)).expect("mkdir");
    }

    pub fn open(&self, name: impl AsRef<Path>) -> File {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.path(name))
            .expect("open scratch file")
    }

    pub fn lines(&self, name: impl AsRef<Path>) -> Vec<String> {
        self.read(name)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

pub struct TicketRecord<'a> {
    pub binding: &'a str,
    pub issued: SystemTime,
    pub presented: SystemTime,
    pub ended: Option<SystemTime>,
    pub last_retryable: Option<&'a str>,
}

pub const GRAPHS: &str =
    "\"verb\":\"search\",\"query\":\"graphs\",\"limit\":10";

/// Writes a pacing file as a gate on another thread or process would.
pub fn write_pacing_at(path: &Path, record: &PacingRecord) {
    let millis = |at: Option<SystemTime>| {
        at.map_or_else(|| "null".to_owned(), |at| millis(at).to_string())
    };
    std::fs::create_dir_all(path.parent().expect("a parent"))
        .expect("mkdir service state");
    std::fs::write(
        path,
        format!(
            "{{\"schema_version\":1,\"last_sent_ms\":{},\
             \"next_free_ms\":{},\"last_finish_ms\":{},\
             \"not_before_ms\":{}}}",
            millis(record.last_sent),
            millis(record.next_free),
            millis(record.last_finish),
            millis(record.not_before)
        ),
    )
    .expect("write pacing");
}

pub fn set_modified(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .or_else(|_| File::open(path))
        .expect("open to date")
        .set_modified(modified)
        .expect("set modified time");
}
