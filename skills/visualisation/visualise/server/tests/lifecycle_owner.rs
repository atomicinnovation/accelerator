use std::time::Duration;

use accelerator_visualiser::{activity::Activity, lifecycle, shutdown::ShutdownReason};

#[tokio::test]
async fn owner_pid_death_triggers_shutdown() {
    // Spawn a short-lived child process, take its PID, wait for it
    // to exit, then confirm lifecycle's owner check returns
    // OwnerPidExited.
    let child = tokio::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    let _ = child.wait_with_output().await;

    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        pid,
        None, // no start_time recorded — owner_alive falls back to bare PID check
        lifecycle::Settings {
            tick: Duration::from_millis(50),
            idle_limit_ms: i64::MAX,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await.expect("owner-death fires within 3s").expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::OwnerPidExited),
        "expected OwnerPidExited, got {reason:?}"
    );
}
