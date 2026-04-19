//! axum server bootstrap. Binds a random port on 127.0.0.1,
//! writes server-info.json + server.pid once the listener is
//! live, and serves a single placeholder route behind a
//! default-deny middleware stack. Signal handling and
//! owner-PID / idle watches land in later phases.

use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

use crate::config::Config;

/// 1 MiB cap on request bodies. The placeholder route never reads a body,
/// but the cap is the default-deny baseline every later phase inherits.
const REQUEST_BODY_LIMIT: usize = 1_048_576;

/// 30s request timeout — long enough for markdown rendering and diff
/// responses in later phases, short enough that a stuck handler can't
/// pin a worker forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AppState {
    pub cfg: Arc<Config>,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub version: String,
    pub pid: i32,
    /// Process start-time stamp, used for PID-identity checks.
    /// Seconds-since-epoch. `None` on platforms where it can't
    /// be obtained — callers fall back to bare PID comparison.
    pub start_time: Option<u64>,
    pub host: String,
    pub port: u16,
    pub url: String,
    pub log_path: PathBuf,
    pub tmp_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("host {0} is not a loopback address")]
    NonLoopbackHost(String),
    #[error("failed to bind listener on {addr}: {source}")]
    Bind { addr: String, source: std::io::Error },
    #[error("failed to write lifecycle file {path}: {source}")]
    LifecycleWrite { path: PathBuf, source: std::io::Error },
    #[error(transparent)]
    Serve(#[from] std::io::Error),
}

pub async fn run(cfg: Config, info_path: &Path) -> Result<(), ServerError> {
    // Defence-in-depth: refuse any non-loopback host even though
    // the launcher always writes 127.0.0.1.
    let host: IpAddr = cfg
        .host
        .parse()
        .map_err(|_| ServerError::NonLoopbackHost(cfg.host.clone()))?;
    if !host.is_loopback() {
        return Err(ServerError::NonLoopbackHost(cfg.host.clone()));
    }

    let state = Arc::new(AppState {
        cfg: Arc::new(cfg),
    });
    let app = Router::new()
        .route("/", get(placeholder_root))
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(middleware::from_fn(host_header_guard))
        .with_state(state.clone());

    let bind_addr = SocketAddr::new(host, 0);
    let listener = TcpListener::bind(bind_addr).await.map_err(|source| ServerError::Bind {
        addr: bind_addr.to_string(),
        source,
    })?;
    let local = listener.local_addr().map_err(|source| ServerError::Bind {
        addr: bind_addr.to_string(),
        source,
    })?;

    let info = ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        pid: std::process::id() as i32,
        start_time: process_start_time(std::process::id() as i32),
        host: state.cfg.host.clone(),
        port: local.port(),
        url: format!("http://{}:{}", state.cfg.host, local.port()),
        log_path: state.cfg.log_path.clone(),
        tmp_path: state.cfg.tmp_path.clone(),
    };

    // Write PID file first (smaller artefact, faster to land) then
    // server-info.json. Both atomic-rename; order matters only to
    // the launcher's poll-for-readiness, which keys on
    // server-info.json.
    let pid_path = info_path.with_file_name("server.pid");
    write_pid_file(&pid_path, info.pid).map_err(|source| ServerError::LifecycleWrite {
        path: pid_path.clone(),
        source,
    })?;
    write_server_info(info_path, &info).map_err(|source| ServerError::LifecycleWrite {
        path: info_path.to_path_buf(),
        source,
    })?;
    info!(url = %info.url, pid = info.pid, start_time = ?info.start_time, "server-started");

    axum::serve(listener, app).await?;
    Ok(())
}

fn write_server_info(path: &Path, info: &ServerInfo) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "server-info.json path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, info)?;
    tmp.as_file_mut().write_all(b"\n")?;
    // Owner-only read/write — the file reveals listener URL and
    // process identity, and lives under the user's project tree
    // where other local accounts may have traversal.
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

fn write_pid_file(path: &Path, pid: i32) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "server.pid path has no parent",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    writeln!(tmp.as_file_mut(), "{pid}")?;
    use std::os::unix::fs::PermissionsExt;
    tmp.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    tmp.persist(path)?;
    Ok(())
}

/// Seconds-since-epoch at which `pid` started, if obtainable.
/// macOS: `ps -p <pid> -o lstart=` → `date -j -f` → epoch.
/// Linux: `/proc/<pid>/stat` field 22 (clock ticks since boot) +
///        `/proc/stat` `btime` → absolute epoch.
/// Returns None on any parse or IO failure — the caller falls back
/// to bare PID comparison.
pub(crate) fn process_start_time(pid: i32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // Field 22 (1-indexed) is starttime in clock ticks since boot.
        // The command field (field 2) is wrapped in parens and may
        // contain spaces, so skip past the last ')' before splitting.
        let tail = stat.rsplit_once(')').map(|(_, t)| t)?;
        let starttime_ticks: u64 = tail.split_whitespace().nth(19)?.parse().ok()?;
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
        if hz == 0 {
            return None;
        }
        let btime_line = std::fs::read_to_string("/proc/stat")
            .ok()?
            .lines()
            .find(|l| l.starts_with("btime "))?
            .to_string();
        let btime: u64 = btime_line.split_whitespace().nth(1)?.parse().ok()?;
        Some(btime + starttime_ticks / hz)
    }
    #[cfg(target_os = "macos")]
    {
        // Delegate to BSD `date -j -f` so Rust and shell
        // (scripts/_launcher-helpers.sh `start_time_of`) produce
        // byte-identical epoch-seconds. The alternative — parsing
        // the wall-clock components manually — would diverge from
        // the shell whenever the host isn't in UTC, because BSD
        // `date -j` interprets the input in the local timezone.
        let ps = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "lstart="])
            .output()
            .ok()?;
        if !ps.status.success() {
            return None;
        }
        let lstart = String::from_utf8(ps.stdout).ok()?;
        let lstart = lstart.trim();
        if lstart.is_empty() {
            return None;
        }
        let date = std::process::Command::new("date")
            .args(["-j", "-f", "%a %b %d %H:%M:%S %Y", lstart, "+%s"])
            .output()
            .ok()?;
        if !date.status.success() {
            return None;
        }
        let epoch = String::from_utf8(date.stdout).ok()?;
        epoch.trim().parse::<u64>().ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

async fn host_header_guard(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Defence-in-depth against DNS-rebinding: only accept the
    // Host header values we actually bind to.
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let (host_part, _) = host.split_once(':').unwrap_or((host, ""));
    if host_part == "127.0.0.1" || host_part == "localhost" || host_part.is_empty() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

async fn placeholder_root() -> &'static str {
    concat!(
        "accelerator-visualiser ",
        env!("CARGO_PKG_VERSION"),
        " — Phase 2 bootstrap. UI lands in a later phase.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config(tmp: &Path) -> Config {
        Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    #[test]
    fn write_server_info_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let info = ServerInfo {
            version: "0.0.0-test".into(),
            pid: 42,
            start_time: Some(1_700_000_000),
            host: "127.0.0.1".into(),
            port: 1234,
            url: "http://127.0.0.1:1234".into(),
            log_path: dir.path().join("server.log"),
            tmp_path: dir.path().to_path_buf(),
        };
        write_server_info(&info_path, &info).unwrap();
        let bytes = std::fs::read(&info_path).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["port"], 1234);
        assert_eq!(v["url"], "http://127.0.0.1:1234");
        assert_eq!(v["pid"], 42);
        assert_eq!(v["start_time"], 1_700_000_000);
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&info_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "server-info.json must be owner-only");
    }

    #[test]
    fn write_pid_file_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("server.pid");
        write_pid_file(&p, 9999).unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content.trim(), "9999");
        assert!(content.ends_with('\n'));
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn process_start_time_is_stable_for_same_pid() {
        let me = std::process::id() as i32;
        let first = process_start_time(me);
        let second = process_start_time(me);
        assert_eq!(first, second, "start_time must be stable for the same PID");
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(first.is_some(), "start_time should resolve on Linux/macOS");
    }

    #[tokio::test]
    async fn non_loopback_host_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(dir.path());
        cfg.host = "0.0.0.0".into();
        let err = run(cfg, &dir.path().join("server-info.json")).await.unwrap_err();
        assert!(matches!(err, ServerError::NonLoopbackHost(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn serves_placeholder_root_and_writes_info() {
        let dir = tempfile::tempdir().unwrap();
        let info_path = dir.path().join("server-info.json");
        let cfg = minimal_config(dir.path());

        let info_path_clone = info_path.clone();
        let handle = tokio::spawn(async move {
            run(cfg, &info_path_clone).await.unwrap();
        });

        // Poll for server-info.json to appear (bounded).
        let start = std::time::Instant::now();
        loop {
            if info_path.exists() {
                break;
            }
            if start.elapsed().as_secs() > 5 {
                panic!("server-info.json did not appear in 5s");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        let info: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&info_path).unwrap()).unwrap();
        let url = info["url"].as_str().unwrap().to_string();

        let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
        assert!(body.starts_with("accelerator-visualiser "));
        assert!(body.contains("Phase 2 bootstrap"));

        handle.abort();
    }
}
