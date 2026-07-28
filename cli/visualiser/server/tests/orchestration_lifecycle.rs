//! Black-box lifecycle tests driving the real binary's `start|stop|status|serve`
//! against isolated fixture projects. Each test gets a unique tempdir project,
//! a loopback port-0 bind, and an RAII reaper that SIGKILLs any surviving daemon
//! on drop (fires on assertion-panic too) so real detached daemons never leak.

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use assert_cmd::prelude::*;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn state_dir(project: &Path) -> PathBuf {
    project.join(".accelerator/tmp/visualiser")
}

/// Reaps any daemon recorded under a project's state dir when dropped.
struct Reaper {
    project: PathBuf,
}

impl Drop for Reaper {
    fn drop(&mut self) {
        let info = state_dir(&self.project).join("server-info.json");
        if let Ok(bytes) = std::fs::read(info) {
            if let Ok(value) =
                serde_json::from_slice::<serde_json::Value>(&bytes)
            {
                if let Some(pid) = value["pid"].as_i64() {
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
                }
            }
        }
    }
}

/// Create a project with an init sentinel and the given config-frontmatter body.
fn seed_project(dir: &Path, config_body: &str) {
    let acc = dir.join(".accelerator");
    std::fs::create_dir_all(acc.join("tmp")).unwrap();
    std::fs::write(acc.join("tmp/.gitignore"), "").unwrap();
    std::fs::write(acc.join("config.md"), config_body).unwrap();
    std::fs::create_dir_all(dir.join("meta/work")).unwrap();
}

fn cli(project: &Path) -> Command {
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.current_dir(project)
        .env("CLAUDE_PLUGIN_ROOT", repo_root());
    cmd
}

fn port_of(url: &str) -> u16 {
    url.rsplit(':')
        .next()
        .unwrap()
        .trim_end_matches('/')
        .parse()
        .unwrap()
}

fn port_reachable(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(500),
    )
    .is_ok()
}

#[test]
fn status_is_stopped_before_any_start() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    let out = cli(tmp.path()).arg("status").output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "stopped");
}

#[test]
fn start_status_stop_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let _reaper = Reaper {
        project: tmp.path().to_path_buf(),
    };
    seed_project(tmp.path(), "---\nvisualiser:\n  idle_timeout: never\n---\n");

    let start = cli(tmp.path()).arg("start").output().unwrap();
    assert!(
        start.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&start.stderr)
    );
    let printed = String::from_utf8_lossy(&start.stdout);
    assert!(
        printed.contains("**Visualiser URL**: http://127.0.0.1:"),
        "start must print the loopback URL, got: {printed}"
    );

    let status = cli(tmp.path()).arg("status").output().unwrap();
    let v =
        serde_json::from_slice::<serde_json::Value>(&status.stdout).unwrap();
    assert_eq!(v["status"], "running");
    let port = port_of(v["url"].as_str().unwrap());
    assert!(
        port_reachable(port),
        "server must be reachable while running"
    );

    // Reuse short-circuit: a second start returns the same URL.
    let again = cli(tmp.path()).arg("start").output().unwrap();
    assert!(
        String::from_utf8_lossy(&again.stdout).contains(&format!(":{port}"))
    );

    let stop = cli(tmp.path()).arg("stop").output().unwrap();
    assert!(stop.status.success());
    let sv = serde_json::from_slice::<serde_json::Value>(&stop.stdout).unwrap();
    assert_eq!(sv["status"], "stopped");

    let status = cli(tmp.path()).arg("status").output().unwrap();
    let v =
        serde_json::from_slice::<serde_json::Value>(&status.stdout).unwrap();
    assert_eq!(v["status"], "stopped");
    assert!(
        !port_reachable(port),
        "server must be unreachable after stop"
    );
}

#[test]
fn stop_with_no_server_is_not_running() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    let out = cli(tmp.path()).arg("stop").output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "not_running");
}

#[test]
fn start_refuses_uninitialised_project() {
    let tmp = tempfile::tempdir().unwrap();
    // .accelerator exists (so the root is discovered) but no tmp/.gitignore.
    std::fs::create_dir_all(tmp.path().join(".accelerator")).unwrap();
    std::fs::write(tmp.path().join(".accelerator/config.md"), "---\n---\n")
        .unwrap();
    let out = cli(tmp.path()).arg("start").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"], "accelerator not initialised");
    assert!(!state_dir(tmp.path()).join("server-info.json").exists());
}

#[test]
fn stop_refuses_recycled_pid() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    let sd = state_dir(tmp.path());
    std::fs::create_dir_all(&sd).unwrap();
    // Record this live test process with a bogus start-time: alive, but its
    // identity cannot match, so stop must refuse to signal it.
    let me = std::process::id();
    std::fs::write(
        sd.join("server-info.json"),
        format!(
            r#"{{"pid":{me},"start_time":1,"url":"http://127.0.0.1:9","host":"127.0.0.1","port":9,"version":"t","log_path":"x","tmp_path":"x"}}"#
        ),
    )
    .unwrap();

    let out = cli(tmp.path()).arg("stop").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "refused");
    // The test process must still be alive.
    assert!(kill(Pid::from_raw(me as i32), None).is_ok());
    // Stale lifecycle files are cleaned up.
    assert!(!sd.join("server-info.json").exists());
}

#[test]
fn stop_reaps_dead_pid_state() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    let sd = state_dir(tmp.path());
    std::fs::create_dir_all(&sd).unwrap();
    let dead = spawn_and_reap();
    std::fs::write(
        sd.join("server-info.json"),
        format!(
            r#"{{"pid":{dead},"url":"http://127.0.0.1:9","host":"127.0.0.1","port":9,"version":"t","log_path":"x","tmp_path":"x"}}"#
        ),
    )
    .unwrap();

    let status = cli(tmp.path()).arg("status").output().unwrap();
    let sv: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(sv["status"], "stopped", "dead-pid state reads as stopped");

    let out = cli(tmp.path()).arg("stop").output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "stopped");
}

#[test]
fn forced_kill_synthesises_stopped_sentinel() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    let sd = state_dir(tmp.path());
    std::fs::create_dir_all(&sd).unwrap();
    // A process that ignores SIGTERM forces the SIGKILL escalation. No recorded
    // start-time, so the identity check is skipped and stop signals by pid.
    //
    // The fake announces itself only once the trap is installed, and the test
    // waits for that. Racing `stop` against the shell's startup makes SIGTERM
    // fatal after all, and the graceful stop that follows reports no `forced`
    // flag — a failure that needs a loaded machine to reproduce.
    let ready = sd.join("fake-trap-installed");
    let child = Command::new("sh")
        .args([
            "-c",
            r#"trap '' TERM; : >"$1"; sleep 30"#,
            "sh",
            ready.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !ready.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "the fake never installed its TERM trap",
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let pid = child.id();
    std::fs::write(
        sd.join("server-info.json"),
        format!(
            r#"{{"pid":{pid},"url":"http://127.0.0.1:9","host":"127.0.0.1","port":9,"version":"t","log_path":"x","tmp_path":"x"}}"#
        ),
    )
    .unwrap();

    // The fake is this test process's direct child, so it would zombie (and
    // `kill(pid,0)` still report it alive) after the SIGKILL unless reaped
    // concurrently. A real daemon is init-reaped; reap here to mirror that.
    let reaper = std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    let out = cli(tmp.path()).arg("stop").output().unwrap();
    reaper.join().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "stopped");
    assert_eq!(v["forced"], true);
    let stopped: serde_json::Value = serde_json::from_slice(
        &std::fs::read(sd.join("server-stopped.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stopped["reason"], "forced-sigkill");
    assert!(!sd.join("server-info.json").exists());
}

#[test]
fn serve_rejects_invalid_idle_timeout() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\nvisualiser:\n  idle_timeout: soon\n---\n");
    // serve fails fast at boot (resolve_idle_limit_ms) with exit 1, rather than
    // the exit-2 of a config-composition/load failure.
    cli(tmp.path())
        .args(["serve", "--owner-pid", "0"])
        .assert()
        .code(1);
}

#[test]
fn serve_without_plugin_root_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path(), "---\n---\n");
    Command::cargo_bin("accelerator-visualiser")
        .unwrap()
        .current_dir(tmp.path())
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .args(["serve", "--owner-pid", "0"])
        .assert()
        .code(2);
}

/// Spawn a trivial process, wait for it, and return its now-dead pid.
fn spawn_and_reap() -> u32 {
    let mut child = Command::new("sh").args(["-c", "exit 0"]).spawn().unwrap();
    let pid = child.id();
    let _ = child.wait();
    pid
}
