use std::time::Duration;

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

#[tokio::test]
async fn sigterm_removes_info_writes_stopped_and_exits() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("server.log");
    let cfg_path = tmp.path().join("config.json");
    let config = serde_json::json!({
        "plugin_root": tmp.path(),
        "plugin_version": "0.0.0-test",
        "project_root": tmp.path(),
        "tmp_path": tmp.path(),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": log,
        "doc_paths": {},
        "templates": {}
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = tokio::process::Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap()])
        .spawn()
        .expect("spawn");

    let info_path = tmp.path().join("server-info.json");
    let stopped_path = tmp.path().join("server-stopped.json");
    let start = std::time::Instant::now();
    loop {
        if info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().await.ok();
            panic!("server did not start in 5s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    kill(Pid::from_raw(child.id().unwrap() as i32), Signal::SIGTERM).expect("send SIGTERM");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("server exits on SIGTERM within 30s")
        .expect("wait");
    assert!(status.success(), "server exited with non-zero: {status:?}");

    let pid_path = tmp.path().join("server.pid");
    assert!(!info_path.exists(), "server-info.json must be removed");
    assert!(!pid_path.exists(), "server.pid must be removed");
    assert!(stopped_path.exists(), "server-stopped.json must be written");
    let stopped: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&stopped_path).unwrap()).unwrap();
    assert_eq!(stopped["reason"], "sigterm");
}

#[tokio::test]
async fn server_writes_pid_file_with_its_own_pid() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("config.json");
    let config = serde_json::json!({
        "plugin_root": tmp.path(),
        "plugin_version": "0.0.0-test",
        "project_root": tmp.path(),
        "tmp_path": tmp.path(),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": tmp.path().join("server.log"),
        "doc_paths": {},
        "templates": {}
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = tokio::process::Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap()])
        .spawn()
        .expect("spawn");
    let child_pid = child.id().unwrap() as i32;

    let pid_path = tmp.path().join("server.pid");
    let info_path = tmp.path().join("server-info.json");
    let start = std::time::Instant::now();
    loop {
        if pid_path.exists() && info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill().await.ok();
            panic!("lifecycle files did not appear in 30s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let pid_str = std::fs::read_to_string(&pid_path).unwrap();
    let recorded_pid: i32 = pid_str.trim().parse().unwrap();
    assert_eq!(
        recorded_pid, child_pid,
        "server.pid must match the child's PID"
    );

    child.kill().await.ok();
    let _ = child.wait().await;
}

#[tokio::test]
async fn shutdown_preserves_state_on_stopped_write_failure() {
    // Pre-create a blocker directory at the server-stopped.json path before
    // the binary starts. The server should still exit cleanly (exit 0), and
    // server-info.json + server.pid must remain on disk (the launcher's
    // stale-PID reuse path will reap them on next invocation).
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("config.json");
    let config = serde_json::json!({
        "plugin_root": tmp.path(),
        "plugin_version": "0.0.0-test",
        "project_root": tmp.path(),
        "tmp_path": tmp.path(),
        "host": "127.0.0.1",
        "owner_pid": 0,
        "log_path": tmp.path().join("server.log"),
        "doc_paths": {},
        "templates": {}
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&config).unwrap()).unwrap();

    // Pre-create a non-empty directory at the stopped-file path to force
    // the atomic rename to fail on shutdown.
    let stopped_path = tmp.path().join("server-stopped.json");
    std::fs::create_dir(&stopped_path).unwrap();
    std::fs::write(stopped_path.join("blocker"), "x").unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = tokio::process::Command::new(bin)
        .args(["--config", cfg_path.to_str().unwrap()])
        .spawn()
        .expect("spawn");

    let info_path = tmp.path().join("server-info.json");
    let start = std::time::Instant::now();
    loop {
        if info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().await.ok();
            panic!("server did not start in 5s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    kill(Pid::from_raw(child.id().unwrap() as i32), Signal::SIGTERM).expect("send SIGTERM");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("server exits within 30s")
        .expect("wait");
    assert!(
        status.success(),
        "server must exit 0 even when stopped-write fails: {status:?}"
    );

    let pid_path = tmp.path().join("server.pid");
    assert!(
        info_path.exists(),
        "server-info.json must be preserved when stopped-write fails"
    );
    assert!(
        pid_path.exists(),
        "server.pid must be preserved when stopped-write fails"
    );
}
