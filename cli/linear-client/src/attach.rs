//! Attachment creation, transcribed from `linear-attach-flow.sh`.
//!
//! Link mode is one `attachmentCreate` mutation. Binary mode is three steps —
//! `fileUpload` for a pre-signed URL, a raw PUT of the bytes to that URL
//! (through the unauthenticated [`crate::upload`] transport), then
//! `attachmentCreate` with the resulting `assetUrl`. Both server-supplied URLs
//! are validated before any byte moves, and a step-3 failure after a successful
//! PUT surfaces as an orphaned-asset error, since the three steps are not
//! atomic.

use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use serde_json::json;
use serde_json::Value;

use crate::client::LinearClient;
use crate::surface::interpret;
use crate::surface::SurfaceError;
use crate::upload::url_is_allowed;
use crate::upload::EchoedHeader;

/// Linear's default per-file limit; over it the upload may be rejected, so a
/// warning is emitted, matching `linear-attach-flow.sh:310`.
const SIZE_WARN: u64 = 10 * 1024 * 1024;

const ATTACHMENT_CREATE: &str = "mutation($input: AttachmentCreateInput!) {
    attachmentCreate(input: $input) { success attachment { id } }
  }";

const FILE_UPLOAD: &str =
    "mutation($contentType: String!, $filename: String!, $size: Int!) {
    fileUpload(contentType: $contentType, filename: $filename, size: $size) {
      success
      uploadFile { uploadUrl assetUrl headers { key value } }
    }
  }";

/// The bytes and metadata of a file being attached.
struct FileContent {
    filename: String,
    content_type: &'static str,
    bytes: Vec<u8>,
}

impl LinearClient {
    /// Attaches a link to an issue.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::BadLinkUrl`] for a non-http(s) URL, plus the request
    /// errors.
    pub fn attach_link(
        &self,
        identifier: &str,
        url: &str,
        title: Option<&str>,
    ) -> Result<Value, SurfaceError> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(SurfaceError::BadLinkUrl {
                url: url.to_owned(),
            });
        }
        let title = title.unwrap_or(url);
        self.attachment_create(identifier, title, url)
    }

    /// Attaches a binary file, uploading its bytes and registering the asset.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a refused file, a refused server URL, a failed
    /// upload, or a registration that failed after a successful upload (an
    /// orphaned asset).
    pub fn attach_file(
        &self,
        identifier: &str,
        path: &Path,
        title: Option<&str>,
    ) -> Result<Value, SurfaceError> {
        let content = read_file(path)?;
        let title = title.unwrap_or(&content.filename).to_owned();

        let variables = json!({
            "contentType": content.content_type,
            "filename": content.filename,
            "size": content.bytes.len(),
        });
        let received = self.transport().send(FILE_UPLOAD, &variables)?;
        let body = interpret(&received, "fileUpload")?;
        let upload = upload_target(&body)?;

        let loopback = self.upload().allows_loopback();
        if !url_is_allowed(&upload.upload_url, loopback) {
            return Err(SurfaceError::BadUploadUrl {
                role: "uploadUrl",
                host: host_or_raw(&upload.upload_url),
            });
        }
        if !url_is_allowed(&upload.asset_url, loopback) {
            return Err(SurfaceError::BadUploadUrl {
                role: "assetUrl",
                host: host_or_raw(&upload.asset_url),
            });
        }

        self.upload().put(
            &upload.upload_url,
            content.content_type,
            &upload.headers,
            &content.bytes,
        )?;

        self.register_uploaded(identifier, &title, &upload.asset_url)
    }

    fn attachment_create(
        &self,
        identifier: &str,
        title: &str,
        url: &str,
    ) -> Result<Value, SurfaceError> {
        let variables = json!({
            "input": { "issueId": identifier, "title": title, "url": url },
        });
        let received = self.transport().send(ATTACHMENT_CREATE, &variables)?;
        interpret(&received, "attachmentCreate")
    }

    /// Step 3: register the uploaded asset. A failure here leaves the asset
    /// orphaned in Linear — the three steps are not atomic — so the error names
    /// that state and it is warned as well as returned, since a later run must
    /// reconcile it.
    fn register_uploaded(
        &self,
        identifier: &str,
        title: &str,
        asset_url: &str,
    ) -> Result<Value, SurfaceError> {
        self.attachment_create(identifier, title, asset_url)
            .map_err(|error| {
                let redacted = crate::upload::redact(asset_url);
                tracing::warn!(
                    asset_url = %redacted,
                    "linear attach: the file uploaded but registration failed \
                     — the asset is orphaned"
                );
                SurfaceError::RegisterFailed {
                    asset_url: redacted,
                    detail: error.to_string(),
                }
            })
    }

    pub(crate) const fn upload(&self) -> &crate::upload::UploadTransport {
        self.upload_transport()
    }
}

/// The `fileUpload` response's pre-signed destination and echoed headers.
struct UploadTarget {
    upload_url: String,
    asset_url: String,
    headers: Vec<EchoedHeader>,
}

fn upload_target(body: &Value) -> Result<UploadTarget, SurfaceError> {
    let file =
        body.pointer("/data/fileUpload/uploadFile").ok_or_else(|| {
            SurfaceError::BadResponse {
                operation: "fileUpload",
                reason: "the response carried no uploadFile".to_owned(),
            }
        })?;
    let field = |name: &str| -> Result<String, SurfaceError> {
        file.get(name)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| SurfaceError::BadResponse {
                operation: "fileUpload",
                reason: format!("the response carried no {name}"),
            })
    };
    let headers = file
        .get("headers")
        .and_then(Value::as_array)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|header| {
                    let name = header.get("key").and_then(Value::as_str)?;
                    let value = header.get("value").and_then(Value::as_str)?;
                    Some(EchoedHeader {
                        name: name.to_owned(),
                        value: value.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(UploadTarget {
        upload_url: field("uploadUrl")?,
        asset_url: field("assetUrl")?,
        headers,
    })
}

fn read_file(path: &Path) -> Result<FileContent, SurfaceError> {
    let refuse = |reason: String| SurfaceError::FileRefused {
        path: path.display().to_string(),
        reason,
    };

    let mut file = File::open(path).map_err(|error| {
        refuse(format!("not found or not readable: {error}"))
    })?;
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
            "attachment exceeds Linear's 10 MB default limit; the upload may \
             be rejected"
        );
    }

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| refuse(format!("could not be read: {error}")))?;
    let content_type = tracker_support::sniff(&bytes);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| refuse("has no filename".to_owned()))?
        .to_owned();

    Ok(FileContent {
        filename,
        content_type,
        bytes,
    })
}

fn host_or_raw(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}
