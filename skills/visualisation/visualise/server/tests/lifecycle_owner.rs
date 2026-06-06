use std::time::Duration;

use accelerator_visualiser::{
    activity::Activity, config::DISABLED_IDLE_LIMIT_MS, lifecycle, shutdown::ShutdownReason,
};

#[tokio::test]
async fn owner_pid_death_triggers_shutdown() {
    // Spawn a short-lived child process, take its PID, wait for it
    // to exit, then confirm lifecycle's owner check returns
    // OwnerPidExited.
    let child = tokio::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
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
            // Disable idle so this test isolates the owner trigger; the named
            // sentinel keeps the disable contract in one place (config.rs).
            idle_limit_ms: DISABLED_IDLE_LIMIT_MS,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("owner-death fires within 3s")
        .expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::OwnerPidExited),
        "expected OwnerPidExited, got {reason:?}"
    );
}

#[tokio::test]
async fn owner_exit_still_fires_while_idle_disabled() {
    // AC5/AC7: with idle disabled the server must STILL exit on owner-process
    // exit. The disable smoke test in lifecycle_idle.rs uses owner_pid 0, which
    // short-circuits the owner check and proves nothing about this clause — so
    // this test pairs a dead owner PID with the disable sentinel.
    let child = tokio::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
    let pid = child.id().unwrap() as i32;
    let _ = child.wait_with_output().await;

    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        pid,
        None,
        lifecycle::Settings {
            tick: Duration::from_millis(50),
            idle_limit_ms: DISABLED_IDLE_LIMIT_MS,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("owner-death fires within 3s even with idle disabled")
        .expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::OwnerPidExited),
        "expected OwnerPidExited, got {reason:?}"
    );
}
