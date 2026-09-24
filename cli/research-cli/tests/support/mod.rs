//! Runs `accelerator-research` as a subprocess against a mock, through the
//! `test-loopback` API URL seam and the non-sleeping clock. Only compiled into
//! tests gated on that feature.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use http_test_support::MockServer;
use vcs_test_support::hermetic::Hermetic;

pub const SELECT: &str = "select=id,doi,display_name,type,authorships,\
                          primary_location,is_retracted,\
                          abstract_inverted_index";

pub fn openalex_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../research-adapters/tests/fixtures/openalex")
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

/// Runs `fetch <args>` in `project` against `server`, with every OpenAlex key
/// variable cleared unless `env` sets it.
pub fn fetch(
    project: &Project,
    server: &MockServer,
    args: &[&str],
    env: &[(&str, &str)],
) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-research"));
    command
        .arg("fetch")
        .args(args)
        .current_dir(project.root())
        .env("ACCELERATOR_OPENALEX_API_URL", server.base_url())
        .env("ACCELERATOR_RESEARCH_TEST_CLOCK_LOG", project.clock_log())
        .env_remove("ACCELERATOR_OPENALEX_API_KEY")
        .env_remove("ACCELERATOR_OPENALEX_API_KEY_CMD")
        .env_remove("ACCELERATOR_ALLOW_INSECURE_LOCAL")
        .stdin(Stdio::null());
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
