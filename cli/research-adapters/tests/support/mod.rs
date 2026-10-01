//! A virtual clock, captured diagnostics, and a scratch project directory
//! for the file-backed adapters' suites.

#![allow(dead_code, clippy::expect_used)]

use std::cell::Cell;
use std::cell::RefCell;
use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::fetch::Clock;
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
