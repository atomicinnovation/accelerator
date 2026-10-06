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
use research_adapters::diagnostics::Diagnostics;
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

    pub fn queue_dir(&self) -> ScratchDir {
        self.dir().nested("arxiv-queue")
    }

    pub fn queue_directory(&self) -> PathBuf {
        self.directory().join("arxiv-queue")
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

    pub fn age_file(&self, name: &str, modified: SystemTime) {
        set_modified(&self.queue_path(name), modified);
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

    fn path(&self, name: &str) -> PathBuf {
        let directory = self.directory();
        std::fs::create_dir_all(&directory).expect("mkdir scratch");
        directory.join(name)
    }

    pub fn write(&self, name: &str, contents: &str) {
        std::fs::write(self.path(name), contents).expect("write scratch file");
    }

    pub fn read(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.path(name)).ok()
    }

    pub fn mkdir(&self, name: &str) {
        std::fs::create_dir_all(self.path(name)).expect("mkdir");
    }

    pub fn open(&self, name: &str) -> File {
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.path(name))
            .expect("open scratch file")
    }

    pub fn lines(&self, name: &str) -> Vec<String> {
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
