//! Meta visualiser server entry point.
//!
//! Phase 2 hosts only the bootstrap: read the config.json path
//! from argv, initialise tracing, bind a listener, write
//! server-info.json, and wait on shutdown signals. Indexing,
//! file-watching, SSE, and API routes all land in later phases.

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "accelerator-visualiser starting");
    ExitCode::SUCCESS
}
