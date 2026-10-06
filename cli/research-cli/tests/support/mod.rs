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
use vcs_test_support::hermetic::Hermetic;

pub const SELECT: &str = "select=id,doi,display_name,type,authorships,\
                          primary_location,is_retracted,\
                          abstract_inverted_index";

/// Milliseconds since the Unix epoch at which every test clock's wall time
/// starts, so values one process persists compare deterministically with
/// another's.
pub const CLOCK_EPOCH_MS: &str = "1800000000000";

pub const CALL_BUDGET_MS: &str = "ACCELERATOR_RESEARCH_TEST_CALL_BUDGET_MS";

/// The stored arguments of a search, as a ticket record holds them.
pub fn search_binding(query: &str, limit: u8) -> String {
    format!("\"verb\":\"search\",\"query\":{query:?},\"limit\":{limit}")
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

fn epoch_ms() -> u64 {
    CLOCK_EPOCH_MS.parse().expect("an epoch in milliseconds")
}

pub fn openalex_fixture(name: &str) -> String {
    adapter_fixture("openalex", name)
}

pub fn arxiv_fixture(name: &str) -> String {
    adapter_fixture("arxiv", name)
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

    /// The directory the arXiv lock, pacing state and logs live in under the
    /// default `paths.tmp`.
    pub fn research_scratch(&self) -> PathBuf {
        self.root.join(".accelerator/tmp/research")
    }

    pub fn queue_dir(&self) -> PathBuf {
        let queue = self.research_scratch().join("arxiv-queue");
        std::fs::create_dir_all(&queue).expect("mkdir arxiv-queue");
        queue
    }

    pub fn seed_ticket(&self, ticket: &str, seeded: &SeededTicket<'_>) {
        self.seed_ticket_before(ticket, seeded, epoch_ms());
    }

    /// Seeds a ticket for calls on the real clock, relative to now.
    pub fn seed_ticket_now(&self, ticket: &str, seeded: &SeededTicket<'_>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("after the epoch")
            .as_millis();
        self.seed_ticket_before(
            ticket,
            seeded,
            u64::try_from(now).expect("milliseconds fit"),
        );
    }

    fn seed_ticket_before(
        &self,
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
            self.queue_dir().join(format!("{ticket}.json")),
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

    /// Holds `name` in the research scratch directory exclusively, as
    /// another call would.
    pub fn hold(&self, name: &str) -> File {
        let path = self.research_scratch().join(name);
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("mkdir");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .expect("open to hold");
        rustix::fs::flock(
            &file,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .expect("hold the lock");
        file
    }

    pub fn hold_ticket(&self, ticket: &str) -> File {
        self.hold(&format!("arxiv-queue/{ticket}.lock"))
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
        let mut tickets: Vec<(u64, String)> =
            std::fs::read_dir(self.queue_dir())
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
        std::fs::read_to_string(
            self.research_scratch().join("arxiv-contention.log"),
        )
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
