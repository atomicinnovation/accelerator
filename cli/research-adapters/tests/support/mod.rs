//! A virtual clock, captured diagnostics, and a scratch project directory
//! for the file-backed adapters' suites.

#![allow(dead_code, clippy::expect_used)]

use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
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
use research_adapters::contention::FileContentionLog;
use research_adapters::diagnostics::Diagnostics;
use research_adapters::pacing::FilePacingGate;
use research_adapters::queue::FileArxivQueue;
use research_adapters::scratch::ScratchDir;

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

/// A file or directory of arXiv's queue and pacing state.
#[derive(Debug, Clone, Copy)]
pub enum StateFile {
    ServingLock,
    Pacing,
    RequestLog,
    ContentionLog,
    Queue,
}

impl StateFile {
    const fn arxiv_name(self) -> &'static str {
        match self {
            Self::ServingLock => "arxiv.lock",
            Self::Pacing => "arxiv-pacing",
            Self::RequestLog => "arxiv-requests.log",
            Self::ContentionLog => "arxiv-contention.log",
            Self::Queue => "arxiv-queue",
        }
    }
}

/// The pacing state arXiv's gate persists between calls.
#[derive(Default)]
pub struct PacingRecord {
    pub last_sent: Option<SystemTime>,
    pub last_finish: Option<SystemTime>,
    pub not_before: Option<SystemTime>,
}

pub fn arxiv_gate(
    scratch: &Scratch,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> FilePacingGate {
    FilePacingGate::new(scratch.dir(), clock, diagnostics)
}

pub fn arxiv_contention_log(
    scratch: &Scratch,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
) -> FileContentionLog {
    FileContentionLog::new(scratch.dir(), clock, diagnostics)
}

pub fn arxiv_queue(
    research: &ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
    nonces: Box<dyn Fn() -> Nonce>,
) -> FileArxivQueue {
    FileArxivQueue::new(research, clock, diagnostics, nonces)
}

/// Joins `queue` as an arXiv call would.
pub fn join_queue<'q>(
    queue: &'q FileArxivQueue,
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

    pub fn arxiv_state(&self, file: StateFile) -> PathBuf {
        self.directory().join(file.arxiv_name())
    }

    pub fn queue_directory(&self) -> PathBuf {
        self.arxiv_state(StateFile::Queue)
    }

    pub fn write_pacing(&self, record: &PacingRecord) {
        let millis = |at: Option<SystemTime>| {
            at.map_or_else(|| "null".to_owned(), |at| millis(at).to_string())
        };
        self.write(
            self.arxiv_state(StateFile::Pacing),
            &format!(
                "{{\"last_sent_ms\":{},\"last_finish_ms\":{},\
                 \"not_before_ms\":{}}}",
                millis(record.last_sent),
                millis(record.last_finish),
                millis(record.not_before)
            ),
        );
    }

    /// Writes pacing state in the shape stored before sends were recorded.
    pub fn write_pacing_without_last_sent(&self, last_finish: SystemTime) {
        self.write(
            self.arxiv_state(StateFile::Pacing),
            &format!(
                "{{\"last_finish_ms\":{},\"not_before_ms\":null}}",
                millis(last_finish)
            ),
        );
    }

    pub fn stored_pacing(&self) -> serde_json::Value {
        serde_json::from_str(
            &self
                .read(self.arxiv_state(StateFile::Pacing))
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

pub fn set_modified(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .or_else(|_| File::open(path))
        .expect("open to date")
        .set_modified(modified)
        .expect("set modified time");
}
