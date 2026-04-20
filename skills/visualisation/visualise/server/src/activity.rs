//! Tracks the timestamp of the most recent HTTP activity. Consumed
//! by the idle-timeout watch. Middleware updates the atomic on every
//! request; no request-path changes needed.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use axum::{extract::Request, middleware::Next, response::Response};

pub struct Activity(AtomicI64);

impl Activity {
    pub fn new() -> Self {
        Self(AtomicI64::new(now_millis()))
    }
    pub fn touch(&self) {
        self.0.store(now_millis(), Ordering::Relaxed);
    }
    pub fn last_millis(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn middleware(
    state: axum::extract::State<Arc<Activity>>,
    req: Request,
    next: Next,
) -> Response {
    state.touch();
    next.run(req).await
}
