//! The visualiser's runtime state directory and the lifecycle files within it.
//! Owned exclusively by the visualiser (no other tool writes here), which is
//! what makes the stale-temp sweep on `start` safe.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::shutdown::ShutdownReason;

/// The recorded identity of a server as read back from `server-info.json`.
pub struct RecordedServer {
    pub pid: i32,
    pub start_time: Option<u64>,
    pub url: Option<String>,
}

#[derive(Deserialize)]
struct InfoFile {
    pid: i32,
    #[serde(default)]
    start_time: Option<u64>,
    #[serde(default)]
    url: Option<String>,
}

/// The visualiser state directory: `<project>/<paths.tmp>/visualiser`.
pub struct StateDir {
    dir: PathBuf,
}

impl StateDir {
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.dir
    }

    #[must_use]
    pub fn info_path(&self) -> PathBuf {
        self.dir.join("server-info.json")
    }

    #[must_use]
    pub fn pid_path(&self) -> PathBuf {
        self.dir.join("server.pid")
    }

    #[must_use]
    pub fn stopped_path(&self) -> PathBuf {
        self.dir.join("server-stopped.json")
    }

    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.dir.join("launcher.lock")
    }

    #[must_use]
    pub fn bootstrap_log_path(&self) -> PathBuf {
        self.dir.join("server.bootstrap.log")
    }

    /// The recorded server identity, or `None` when `server-info.json` is absent
    /// or unparseable (the server never started, or was cleanly stopped).
    #[must_use]
    pub fn read_server(&self) -> Option<RecordedServer> {
        let bytes = std::fs::read(self.info_path()).ok()?;
        let info: InfoFile = serde_json::from_slice(&bytes).ok()?;
        Some(RecordedServer {
            pid: info.pid,
            start_time: info.start_time,
            url: info.url,
        })
    }

    /// Remove `server-info.json` and `server.pid`, ignoring absence.
    pub fn remove_lifecycle_files(&self) {
        let _ = std::fs::remove_file(self.info_path());
        let _ = std::fs::remove_file(self.pid_path());
    }

    /// Synthesise `server-stopped.json` for a forced kill (the daemon never
    /// wrote its own sentinel), mirroring the server's clean-shutdown invariant.
    pub fn write_forced_stopped(&self) -> std::io::Result<()> {
        crate::server::write_server_stopped(
            &self.stopped_path(),
            ShutdownReason::ForcedSigkill,
        )
    }

    pub fn remove_stopped(&self) {
        let _ = std::fs::remove_file(self.stopped_path());
    }

    /// Remove leaked atomic-write temp files (`store::TEMP_PREFIX`) left in the
    /// state dir by a crashed writer. Confined to this visualiser-exclusive
    /// directory and only ever called once no live server has been confirmed.
    pub fn clean_stale_temps(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(store::TEMP_PREFIX)
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
