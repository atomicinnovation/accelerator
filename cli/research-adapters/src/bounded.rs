//! Blocking on async work under a budget that covers it whole, and reading a
//! body no further than a cap.

use std::future::Future;
use std::io;
use std::time::Duration;

use tokio::runtime::Runtime;

/// # Errors
///
/// The I/O error when the runtime's driver cannot be set up.
pub fn current_thread_runtime() -> io::Result<Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// `work`'s own answer, or `timed_out` once `budget` passes first.
pub fn within<T>(
    runtime: &Runtime,
    budget: Duration,
    work: impl Future<Output = T>,
    timed_out: T,
) -> T {
    runtime.block_on(async {
        tokio::time::timeout(budget, work)
            .await
            .unwrap_or(timed_out)
    })
}

/// At most the first `cap` bytes of a body, and whether more followed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CappedBody {
    pub head: Vec<u8>,
    pub overflowed: bool,
}

/// Reads no further than the chunk that crosses `cap`.
///
/// # Errors
///
/// The transfer's error when the body breaks off before it ends or crosses
/// `cap`.
pub async fn read_capped(
    mut response: reqwest::Response,
    cap: usize,
) -> Result<CappedBody, reqwest::Error> {
    let mut head = Vec::new();
    while head.len() <= cap {
        match response.chunk().await? {
            Some(chunk) => head.extend_from_slice(&chunk),
            None => break,
        }
    }
    let overflowed = head.len() > cap;
    head.truncate(cap);
    Ok(CappedBody { head, overflowed })
}
