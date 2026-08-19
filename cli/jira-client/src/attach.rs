//! Attachment upload: one `multipart/form-data` POST carrying
//! `X-Atlassian-Token: no-check`, with one part per file.
//!
//! Each file is opened once and its type and size come from that handle's own
//! metadata, closing the check-then-read TOCTOU window in which a link could
//! be repointed at a secret between a separate stat and open. Attachment paths
//! are confined to the repository root so a relative path cannot address an
//! arbitrary readable file.

use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use reqwest::Method;
use serde_json::Value;

use crate::client::JiraClient;
use crate::multipart;
use crate::multipart::Part;
use crate::surface::SurfaceError;

/// Jira Cloud's default per-file limit; over it the upload may be rejected, so
/// a warning is emitted.
const SIZE_WARN: u64 = 10 * 1024 * 1024;

impl JiraClient {
    /// Uploads one or more files as attachments on an issue.
    ///
    /// Files are confined to `root`; each is opened once and its type and size
    /// checked on that handle.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused key, a path outside `root`, a non-regular
    /// file, an unreadable file, a transport failure or a non-2xx status.
    pub fn attach(
        &self,
        key: &str,
        files: &[&Path],
        root: &Path,
    ) -> Result<Value, SurfaceError> {
        let parts = files
            .iter()
            .map(|path| read_part(path, root))
            .collect::<Result<Vec<_>, _>>()?;

        let boundary = multipart::boundary_free_of(&parts)?;
        let body = multipart::encode(&boundary, &parts)?;
        let content_type = format!("multipart/form-data; boundary={boundary}");
        let request_path = Self::issue_path(key, "/attachments")?;
        let received = self.transport().send_bytes(
            &Method::POST,
            &request_path,
            &content_type,
            &[("X-Atlassian-Token", "no-check")],
            &body,
        )?;
        if received.status < 200 || received.status >= 300 {
            return Err(SurfaceError::status(
                "attach",
                received.status,
                received.body,
            ));
        }
        if received.body.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&received.body).map_err(|error| {
            SurfaceError::BadResponse {
                operation: "attach",
                reason: format!("the response was not JSON: {error}"),
            }
        })
    }
}

fn read_part(path: &Path, root: &Path) -> Result<Part, SurfaceError> {
    let refuse = |reason: String| SurfaceError::FileRefused {
        path: path.display().to_string(),
        reason,
    };

    let canonical_root =
        root.canonicalize()
            .map_err(|error| SurfaceError::FileRefused {
                path: root.display().to_string(),
                reason: format!(
                    "the repository root could not be resolved: {error}"
                ),
            })?;
    // Canonicalising resolves symlinks up front; opening the canonical path
    // then does not re-follow a link, so it cannot be repointed under us.
    let resolved = path.canonicalize().map_err(|error| {
        refuse(format!("not found or not readable: {error}"))
    })?;
    if !resolved.starts_with(&canonical_root) {
        return Err(SurfaceError::PathEscapesRoot {
            path: path.display().to_string(),
        });
    }

    let mut file = File::open(&resolved)
        .map_err(|error| refuse(format!("could not be opened: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| refuse(format!("could not be inspected: {error}")))?;
    if !metadata.is_file() {
        return Err(refuse(
            "is not a regular file — a device, socket or directory cannot be \
             attached"
                .to_owned(),
        ));
    }
    if metadata.len() > SIZE_WARN {
        tracing::warn!(
            path = %path.display(),
            bytes = metadata.len(),
            "attachment exceeds Jira Cloud's 10 MB default limit; the upload \
             may be rejected"
        );
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| refuse(format!("could not be read: {error}")))?;
    let content_type = tracker_support::sniff(&bytes);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SurfaceError::BadFilename {
            name: path.display().to_string(),
        })?
        .to_owned();

    Ok(Part {
        filename,
        content_type,
        bytes,
    })
}
