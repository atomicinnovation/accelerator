//! `accelerator design executor` after the pre-flight: the lifecycle of the
//! vendored trees' leases and of the daemon, through the compiled binary.
//!
//! A stub `node` plays both the daemon and the client, so the real
//! `DaemonSpawner` and `ExecClient` run without Playwright, a browser or a
//! network. No Playwright behaviour is exercised here.

use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::time::Duration;
use std::time::Instant;

use design::runtime::platform;
use design::runtime::platform::Support;
use lease::{probe_shared_lease, Liveness};
use rustix::process::getpgid;
use rustix::process::kill_process_group;
use rustix::process::Pid;
use rustix::process::Signal;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-design");
const WARM_HANDOVER: &str =
    env!("CARGO_BIN_EXE_design-cli-fixture-warm-handover");

/// The daemon is detached, so its release is scheduled with no one waiting on
/// it; under a fully loaded instrumented suite that takes seconds.
const RELEASE_BOUND: Duration = Duration::from_secs(60);

const STUB_NODE: &str = r#"#!/bin/sh
case "$2" in
  daemon)
    cat <&3 > /dev/null
    printf '{"pid":%s,"start_time":null,"start_time_source":"writer-unavailable"}' $$ > "$4/server-info.json.tmp"
    mv "$4/server-info.json.tmp" "$4/server-info.json"
    exec sleep 300 ;;
  *) exit 0 ;;
esac
"#;

/// On a host the platform classifier rejects, the executor downgrades before
/// it spawns, so these tests would fail for reasons unrelated to leases.
macro_rules! runtime_or_skip {
    () => {
        if let Support::Unsupported(reason) =
            platform::classify(&design_adapters::platform::observe())
        {
            assert!(
                std::env::var_os("CI").is_none(),
                "the platform classifier rejects this CI host: {reason:?}"
            );
            eprintln!("skipping: the platform classifier rejects this host");
            return Ok(());
        }
    };
}

/// A repository, a plugin root and the two trees with their lease siblings,
/// laid out as the launcher lays them out.
struct Runtime {
    work: tempfile::TempDir,
}

impl Runtime {
    fn new() -> Result<Self, TestError> {
        let runtime = Self {
            work: tempfile::tempdir()?,
        };
        fs::create_dir_all(runtime.repo().join(".git"))?;
        fs::create_dir_all(runtime.plugin())?;
        fs::create_dir_all(runtime.driver_tree())?;
        fs::create_dir_all(runtime.browser_tree())?;
        let node = runtime.driver_tree().join("node");
        fs::write(&node, STUB_NODE)?;
        fs::set_permissions(&node, fs::Permissions::from_mode(0o755))?;
        Ok(runtime)
    }

    fn repo(&self) -> PathBuf {
        self.work.path().join("repo")
    }

    fn plugin(&self) -> PathBuf {
        self.work.path().join("plugin")
    }

    fn cache(&self) -> PathBuf {
        self.work.path().join("cache")
    }

    fn driver_tree(&self) -> PathBuf {
        self.cache().join("driver-tree")
    }

    fn browser_tree(&self) -> PathBuf {
        self.cache().join("browser-tree")
    }

    fn driver_lease(&self) -> PathBuf {
        lease::lease_beside(&self.driver_tree())
    }

    fn browser_lease(&self) -> PathBuf {
        lease::lease_beside(&self.browser_tree())
    }

    fn state_dir(&self) -> PathBuf {
        self.repo()
            .join(".accelerator/tmp/inventory-design-playwright/bundled")
    }

    /// A command with a controlled environment, so a developer's own
    /// `ACCELERATOR_*` settings cannot change the outcome.
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.current_dir(self.repo());
        command.env_clear();
        command.env("PATH", std::env::var("PATH").unwrap_or_default());
        // Coverage instrumentation writes to the path this names; clearing it
        // makes the child litter its profile into the directory under test.
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command.env("HOME", self.repo());
        command.env("ACCELERATOR_PLUGIN_ROOT", self.plugin());
        command
    }

    fn warm_handover(&self, arguments: &[&str]) -> Result<Output, TestError> {
        Ok(self
            .command(WARM_HANDOVER)
            .arg(BIN)
            .arg(self.driver_tree())
            .arg(self.driver_lease())
            .arg(self.browser_tree())
            .arg(self.browser_lease())
            .arg("executor")
            .args(arguments)
            .output()?)
    }

    /// Writes the fake launcher the cold path reaches through
    /// `ACCELERATOR_LAUNCHER_PATH`. Each `cache ensure` call takes the next
    /// scripted answer; calls past the script return the real trees.
    fn fake_launcher(&self, script: &[EnsureCall]) -> Result<(), TestError> {
        let mut answers = String::new();
        for (index, call) in (1..).zip(script) {
            let answer = match call {
                EnsureCall::Trees { driver, browser } => format!(
                    "driver={}; browser={}",
                    quoted(driver),
                    quoted(browser)
                ),
                EnsureCall::FailsWith(cause) => format!(
                    "printf '%s\\n' '{{\"error\":\"cache\",\"cause\":\"{cause}\"}}' >&2; exit 1"
                ),
            };
            writeln!(answers, "  {index}) {answer} ;;")?;
        }
        let body = format!(
            r#"#!/bin/sh
[ "$1 $2" = "cache ensure" ] || exit 64
shift 2
printf '%s\n' "$@" >> {arguments}
call=$(( $(cat {calls} 2>/dev/null || echo 0) + 1 ))
echo "$call" > {calls}
driver={driver}; browser={browser}
case "$call" in
{answers}esac
for artifact in "$@"; do
  case "$artifact" in
    driver) tree=$driver ;;
    browser) tree=$browser ;;
    *) exit 65 ;;
  esac
  printf '%s\t%s\t%s\n' "$artifact" "$tree" "$tree.lease"
done
"#,
            arguments = quoted(&self.ensure_arguments()),
            calls = quoted(&self.ensure_calls_path()),
            driver = quoted(&self.driver_tree()),
            browser = quoted(&self.browser_tree()),
        );
        let launcher = self.launcher();
        fs::write(&launcher, body)?;
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755))?;
        Ok(())
    }

    fn launcher(&self) -> PathBuf {
        self.work.path().join("accelerator")
    }

    fn ensure_arguments(&self) -> PathBuf {
        self.work.path().join("ensure-args")
    }

    fn ensure_calls_path(&self) -> PathBuf {
        self.work.path().join("ensure-calls")
    }

    fn ensure_calls(&self) -> Result<u32, TestError> {
        Ok(fs::read_to_string(self.ensure_calls_path())?
            .trim()
            .parse()?)
    }

    fn gone_tree(&self, artifact: &str) -> PathBuf {
        self.cache().join(format!("reclaimed-{artifact}-tree"))
    }

    fn cold(&self, environment: &[(&str, &Path)]) -> Result<Output, TestError> {
        let mut command = self.command(BIN);
        command.env("ACCELERATOR_LAUNCHER_PATH", self.launcher());
        for (name, value) in environment {
            command.env(name, value);
        }
        Ok(command.args(["executor", "ping"]).output()?)
    }

    fn daemon(&self, output: &Output) -> Result<DaemonGroup, TestError> {
        self.daemon_holding(
            output,
            vec![self.driver_lease(), self.browser_lease()],
        )
    }

    fn daemon_holding(
        &self,
        output: &Output,
        leases: Vec<PathBuf>,
    ) -> Result<DaemonGroup, TestError> {
        DaemonGroup::from_state(&self.state_dir(), leases).map_err(|error| {
            format!("{error}; the executor said: {}", stderr_of(output)).into()
        })
    }

    fn marker_reason(&self) -> Result<Option<String>, TestError> {
        let marker = self.state_dir().join("downgrade-marker.json");
        if !marker.exists() {
            return Ok(None);
        }
        let record: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(marker)?)?;
        Ok(record["reason"].as_str().map(str::to_owned))
    }
}

/// One scripted answer of the fake launcher's `cache ensure`.
enum EnsureCall {
    Trees { driver: PathBuf, browser: PathBuf },
    FailsWith(&'static str),
}

fn quoted(path: &Path) -> String {
    format!("'{}'", path.display())
}

fn downgrade_line(output: &Output) -> Option<String> {
    stderr_of(output)
        .lines()
        .find(|line| line.contains(r#""error":"downgrade""#))
        .map(str::to_owned)
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn held(lease: &Path) -> bool {
    probe_shared_lease(lease) == Liveness::Held
}

/// The detached daemon's process group, which `setsid` makes its own.
struct DaemonGroup {
    pid: Pid,
    leases: Vec<PathBuf>,
    terminated: bool,
}

impl DaemonGroup {
    fn from_state(
        state_dir: &Path,
        leases: Vec<PathBuf>,
    ) -> Result<Self, TestError> {
        let body = fs::read_to_string(state_dir.join("server-info.json"))?;
        let record: serde_json::Value = serde_json::from_str(&body)?;
        let raw = record["pid"]
            .as_i64()
            .ok_or("server-info.json carries no integer pid")?;
        let raw = i32::try_from(raw)?;
        if raw <= 1 {
            return Err(format!("refusing to signal pid {raw}").into());
        }
        let pid = Pid::from_raw(raw).ok_or("a zero pid")?;
        if getpgid(Some(pid))? != pid {
            return Err(format!("pid {raw} does not lead its group").into());
        }
        Ok(Self {
            pid,
            leases,
            terminated: false,
        })
    }

    /// Ends the group and waits for every lease to read free. The leases are
    /// polled rather than the pid, because nothing may reap the orphaned
    /// daemon, and a zombie has already closed its descriptors.
    fn terminate(&mut self) -> Result<(), TestError> {
        kill_process_group(self.pid, Signal::TERM)?;
        let deadline = Instant::now() + RELEASE_BOUND;
        loop {
            let mut any_held = false;
            for lease in &self.leases {
                any_held |= held(lease);
            }
            if !any_held {
                self.terminated = true;
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("a lease was still held after the daemon group \
                            was terminated"
                    .into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for DaemonGroup {
    fn drop(&mut self) {
        if self.terminated {
            return;
        }
        if getpgid(Some(self.pid)).is_ok_and(|group| group == self.pid) {
            let _ = kill_process_group(self.pid, Signal::TERM);
        }
    }
}

#[test]
fn a_warm_daemon_holds_every_lease_after_the_client_exits(
) -> Result<(), TestError> {
    runtime_or_skip!();
    let runtime = Runtime::new()?;

    let output = runtime.warm_handover(&["ping"])?;
    let mut daemon = runtime.daemon(&output)?;

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(
        held(&runtime.driver_lease()),
        "the driver lease is not held once the client has exited"
    );
    assert!(
        held(&runtime.browser_lease()),
        "the browser lease is not held once the client has exited"
    );
    daemon.terminate()
}

#[test]
fn a_cold_daemon_holds_every_lease_after_the_client_exits(
) -> Result<(), TestError> {
    runtime_or_skip!();
    let runtime = Runtime::new()?;
    runtime.fake_launcher(&[])?;

    let output = runtime.cold(&[])?;
    let mut daemon = runtime.daemon(&output)?;

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(
        held(&runtime.driver_lease()),
        "the driver lease is not held once the client has exited"
    );
    assert!(
        held(&runtime.browser_lease()),
        "the browser lease is not held once the client has exited"
    );
    assert_eq!(runtime.ensure_calls()?, 1);
    daemon.terminate()
}

#[test]
fn a_driver_exported_without_its_browser_takes_the_cold_path_and_holds_both(
) -> Result<(), TestError> {
    runtime_or_skip!();
    let runtime = Runtime::new()?;
    runtime.fake_launcher(&[])?;

    let driver = runtime.driver_tree();
    let output = runtime.cold(&[("ACCELERATOR_TREE_DRIVER", &driver)])?;
    let mut daemon = runtime.daemon(&output)?;

    let ensured = fs::read_to_string(runtime.ensure_arguments())?;
    assert_eq!(ensured.lines().collect::<Vec<_>>(), ["driver", "browser"]);
    assert!(
        held(&runtime.driver_lease()),
        "the driver lease is not held"
    );
    assert!(
        held(&runtime.browser_lease()),
        "the browser lease is not held"
    );
    daemon.terminate()
}

#[test]
fn an_unleasable_tree_is_skipped_and_the_others_held() -> Result<(), TestError>
{
    runtime_or_skip!();
    let runtime = Runtime::new()?;
    runtime.fake_launcher(&[])?;
    fs::create_dir_all(runtime.browser_lease())?;

    let output = runtime.cold(&[])?;
    let mut daemon =
        runtime.daemon_holding(&output, vec![runtime.driver_lease()])?;

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(
        held(&runtime.driver_lease()),
        "the driver lease is not held"
    );
    assert_eq!(runtime.ensure_calls()?, 1);
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("warning:"))
        .collect();
    assert_eq!(warnings.len(), 1, "{stderr}");
    assert!(warnings[0].contains("'browser'"), "{stderr}");
    let lease = runtime.browser_lease().display().to_string();
    assert_eq!(warnings[0].matches(&lease).count(), 1, "{stderr}");
    daemon.terminate()
}

#[test]
fn a_tree_reclaimed_before_its_lease_is_ensured_again() -> Result<(), TestError>
{
    runtime_or_skip!();
    let runtime = Runtime::new()?;
    runtime.fake_launcher(&[EnsureCall::Trees {
        driver: runtime.gone_tree("driver"),
        browser: runtime.browser_tree(),
    }])?;

    let output = runtime.cold(&[])?;
    let mut daemon = runtime.daemon(&output)?;

    assert_eq!(runtime.ensure_calls()?, 2);
    assert!(
        held(&runtime.driver_lease()),
        "the driver lease is not held"
    );
    assert!(
        held(&runtime.browser_lease()),
        "the browser lease is not held"
    );
    daemon.terminate()
}

#[test]
fn a_tree_still_gone_after_ensuring_again_downgrades_without_a_marker(
) -> Result<(), TestError> {
    runtime_or_skip!();
    let runtime = Runtime::new()?;
    let gone = EnsureCall::Trees {
        driver: runtime.gone_tree("driver"),
        browser: runtime.browser_tree(),
    };
    let still_gone = EnsureCall::Trees {
        driver: runtime.gone_tree("driver"),
        browser: runtime.browser_tree(),
    };
    runtime.fake_launcher(&[gone, still_gone])?;

    let output = runtime.cold(&[])?;

    assert_eq!(
        downgrade_line(&output).as_deref(),
        Some(r#"{"error":"downgrade","reason":"artifact-unavailable"}"#),
        "{}",
        stderr_of(&output)
    );
    assert_eq!(runtime.ensure_calls()?, 2);
    assert_eq!(runtime.marker_reason()?, None);
    assert!(
        !runtime.state_dir().join("server-info.json").exists(),
        "a daemon was started"
    );
    Ok(())
}

#[test]
fn a_failed_second_ensure_is_classified_as_any_failed_ensure(
) -> Result<(), TestError> {
    runtime_or_skip!();
    let first_failure = Runtime::new()?;
    first_failure
        .fake_launcher(&[EnsureCall::FailsWith("cache-unwritable")])?;
    let expected = first_failure.cold(&[])?;

    let runtime = Runtime::new()?;
    runtime.fake_launcher(&[
        EnsureCall::Trees {
            driver: runtime.gone_tree("driver"),
            browser: runtime.browser_tree(),
        },
        EnsureCall::FailsWith("cache-unwritable"),
    ])?;
    let output = runtime.cold(&[])?;

    assert_eq!(runtime.ensure_calls()?, 2);
    assert!(
        downgrade_line(&expected).is_some(),
        "{}",
        stderr_of(&expected)
    );
    assert_eq!(downgrade_line(&output), downgrade_line(&expected));
    assert!(first_failure.marker_reason()?.is_some());
    assert_eq!(runtime.marker_reason()?, first_failure.marker_reason()?);
    Ok(())
}
