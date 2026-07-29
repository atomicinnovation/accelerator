use std::path::{Path, PathBuf};
use std::time::Duration;

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

/// Seed a minimal Model-1 project and return its visualiser state dir.
fn seed_project(project: &Path) -> PathBuf {
    std::fs::create_dir_all(project.join(".accelerator")).unwrap();
    std::fs::write(project.join(".accelerator/config.md"), "---\n---\n")
        .unwrap();
    project.join(".accelerator/tmp/visualiser")
}

fn spawn_serve(project: &Path) -> tokio::process::Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_accelerator-visualiser"))
        .args(["serve", "--owner-pid", "0"])
        .current_dir(project)
        .env("ACCELERATOR_PLUGIN_ROOT", project)
        .spawn()
        .expect("spawn serve")
}

async fn wait_for(
    path: &Path,
    budget: Duration,
    child: &mut tokio::process::Child,
) {
    let start = std::time::Instant::now();
    while !path.exists() {
        if start.elapsed() > budget {
            child.kill().await.ok();
            panic!("{} did not appear in {budget:?}", path.display());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn sigterm_removes_info_writes_stopped_and_exits() {
    let tmp = tempfile::tempdir().unwrap();
    let state = seed_project(tmp.path());
    let mut child = spawn_serve(tmp.path());

    let info_path = state.join("server-info.json");
    wait_for(&info_path, Duration::from_secs(5), &mut child).await;

    kill(Pid::from_raw(child.id().unwrap() as i32), Signal::SIGTERM)
        .expect("send SIGTERM");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("server exits on SIGTERM within 30s")
        .expect("wait");
    assert!(status.success(), "server exited with non-zero: {status:?}");

    assert!(!info_path.exists(), "server-info.json must be removed");
    assert!(
        !state.join("server.pid").exists(),
        "server.pid must be removed"
    );
    let stopped_path = state.join("server-stopped.json");
    assert!(stopped_path.exists(), "server-stopped.json must be written");
    let stopped: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&stopped_path).unwrap()).unwrap();
    assert_eq!(stopped["reason"], "sigterm");
}

#[tokio::test]
async fn server_writes_pid_file_with_its_own_pid() {
    let tmp = tempfile::tempdir().unwrap();
    let state = seed_project(tmp.path());
    let mut child = spawn_serve(tmp.path());
    let child_pid = child.id().unwrap() as i32;

    let pid_path = state.join("server.pid");
    wait_for(&pid_path, Duration::from_secs(30), &mut child).await;

    let recorded_pid: i32 = std::fs::read_to_string(&pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        recorded_pid, child_pid,
        "server.pid must match the child's PID"
    );

    child.kill().await.ok();
    let _ = child.wait().await;
}

#[tokio::test]
async fn shutdown_preserves_state_on_stopped_write_failure() {
    // A blocker directory at the server-stopped.json path forces the atomic
    // rename to fail on shutdown; the server must still exit 0 and preserve
    // server-info.json + server.pid for the launcher's stale-PID reuse path.
    let tmp = tempfile::tempdir().unwrap();
    let state = seed_project(tmp.path());
    std::fs::create_dir_all(&state).unwrap();
    let stopped_path = state.join("server-stopped.json");
    std::fs::create_dir(&stopped_path).unwrap();
    std::fs::write(stopped_path.join("blocker"), "x").unwrap();

    let mut child = spawn_serve(tmp.path());
    let info_path = state.join("server-info.json");
    wait_for(&info_path, Duration::from_secs(5), &mut child).await;

    kill(Pid::from_raw(child.id().unwrap() as i32), Signal::SIGTERM)
        .expect("send SIGTERM");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("server exits within 30s")
        .expect("wait");
    assert!(
        status.success(),
        "server must exit 0 even when stopped-write fails: {status:?}"
    );

    assert!(
        info_path.exists(),
        "server-info.json must be preserved when stopped-write fails"
    );
    assert!(
        state.join("server.pid").exists(),
        "server.pid must be preserved when stopped-write fails"
    );
}
