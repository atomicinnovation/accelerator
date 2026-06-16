use std::time::Duration;

use visualiser::{
    activity::Activity, config::DISABLED_IDLE_LIMIT_MS, lifecycle,
    shutdown::ShutdownReason,
};

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
        .await
        .expect("idle fires within 3s")
        .expect("channel ok");
    assert!(
        matches!(reason, ShutdownReason::IdleTimeout),
        "expected IdleTimeout, got {reason:?}"
    );
}

#[tokio::test]
async fn disabled_idle_never_fires() {
    // Smoke check: the disable sentinel neutralises the idle trigger so no
    // ShutdownReason arrives within ~15 ticks. The authoritative guarantee is
    // the resolver unit assertion that disable tokens map to the sentinel; this
    // confirms the sentinel is inert against the loop's `idle >= limit` compare.
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        0, // owner_pid 0 skips the owner check
        None,
        lifecycle::Settings {
            tick: Duration::from_millis(20),
            idle_limit_ms: DISABLED_IDLE_LIMIT_MS,
        },
        tx,
    );
    let res = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
    assert!(res.is_err(), "disabled idle must not fire, got {res:?}");
}

#[tokio::test]
async fn idle_survives_below_threshold_then_fires() {
    // Two-sided boundary: still alive comfortably below the threshold, then
    // fires shortly after. Guards against an off-by-tick or fire-unconditionally
    // mutation of `idle >= settings.idle_limit_ms` that a fires-by-deadline test
    // alone would miss.
    let activity = std::sync::Arc::new(Activity::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    lifecycle::spawn(
        activity,
        0,
        None,
        lifecycle::Settings {
            tick: Duration::from_millis(20),
            idle_limit_ms: 400,
        },
        tx,
    );
    // Below the threshold: must still be alive.
    let early =
        tokio::time::timeout(Duration::from_millis(150), rx.recv()).await;
    assert!(
        early.is_err(),
        "must not fire before the threshold, got {early:?}"
    );
    // After the threshold: must fire.
    let late =
        tokio::time::timeout(Duration::from_millis(1000), rx.recv()).await;
    assert!(
        matches!(late, Ok(Some(ShutdownReason::IdleTimeout))),
        "must fire after the threshold, got {late:?}"
    );
}
