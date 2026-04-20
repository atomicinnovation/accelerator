use std::time::Duration;

use accelerator_visualiser::{activity::Activity, lifecycle, shutdown::ShutdownReason};

#[tokio::test]
async fn idle_timeout_fires_with_fast_clock() {
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        0, // owner_pid 0 skips the owner check
        None,
        lifecycle::Settings {
            tick: Duration::from_millis(50),
            idle_limit_ms: 200,
        },
        tx,
    );
    let reason = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await.expect("idle fires within 3s").expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::IdleTimeout),
        "expected IdleTimeout, got {reason:?}"
    );
}
