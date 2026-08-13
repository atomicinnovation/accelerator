//! The pending-push marker's JSON encoding and filesystem path — the
//! adapter half of `work::sync::push_precondition`'s domain types.

use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use sha2::Digest as _;
use sha2::Sha256;
use tracker::ExternalId;
use work::sync::PendingPush;
use work::sync::RequestFingerprint;

#[derive(Debug)]
pub enum MarkerError {
    Io(String),
    Malformed(String),
}

impl std::fmt::Display for MarkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(detail) | Self::Malformed(detail) => {
                write!(formatter, "{detail}")
            }
        }
    }
}

impl std::error::Error for MarkerError {}

/// `<integrations>/<integration>/pending-push/<slug>.json`.
#[must_use]
pub fn path(integrations_dir: &Path, integration: &str, slug: &str) -> PathBuf {
    integrations_dir
        .join(integration)
        .join("pending-push")
        .join(format!("{slug}.json"))
}

/// The request fingerprint's digest.
///
/// Length-prefixed rather than concatenated: undelimited `title + body +
/// kind` is not injective, so `("ab", "c", k)` and `("a", "bc", k)` would
/// collide, and proving two requests are the *same* before adopting a
/// remote id is this value's whole job.
#[must_use]
pub fn request_digest(title: &str, body: &str, kind: &str) -> String {
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    for field in [title, body, kind] {
        hasher.update((field.len() as u64).to_le_bytes());
        hasher.update(field.as_bytes());
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn fingerprint_from(
    object: &serde_json::Map<String, Value>,
) -> Option<RequestFingerprint> {
    Some(RequestFingerprint {
        title: object.get("title")?.as_str()?.to_owned(),
        digest: object.get("digest")?.as_str()?.to_owned(),
        attempted_at: object.get("attempted_at")?.as_u64()?,
        failure: object
            .get("failure")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

/// Parses the marker's persisted JSON.
///
/// `Ok(None)` for absent content and `Err` for unparseable content stay
/// distinct, so a crash mid-write cannot read as "no previous attempt" and
/// re-issue a non-idempotent `create`.
///
/// # Errors
///
/// [`MarkerError::Malformed`] when `content` is present but not a valid
/// marker.
pub fn read(content: Option<&str>) -> Result<Option<PendingPush>, MarkerError> {
    let Some(raw) = content else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| MarkerError::Malformed(error.to_string()))?;
    let object = value.as_object().ok_or_else(|| {
        MarkerError::Malformed("not a JSON object".to_owned())
    })?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| MarkerError::Malformed("missing 'kind'".to_owned()))?;
    let request = fingerprint_from(object).ok_or_else(|| {
        MarkerError::Malformed(
            "missing or malformed fingerprint fields".to_owned(),
        )
    })?;

    match kind {
        "attempted" => Ok(Some(PendingPush::Attempted { request })),
        "created" => {
            let external_id = object
                .get("external_id")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MarkerError::Malformed(
                        "created marker missing 'external_id'".to_owned(),
                    )
                })?;
            Ok(Some(PendingPush::Created {
                request,
                external_id: ExternalId::new(external_id.to_owned()),
            }))
        }
        other => Err(MarkerError::Malformed(format!(
            "unrecognised marker kind: {other}"
        ))),
    }
}

#[must_use]
pub fn render(marker: &PendingPush) -> String {
    let (kind, request, external_id) = match marker {
        PendingPush::Attempted { request } => ("attempted", request, None),
        PendingPush::Created {
            request,
            external_id,
        } => ("created", request, Some(external_id)),
    };
    let mut object = serde_json::Map::new();
    object.insert("kind".to_owned(), Value::String(kind.to_owned()));
    object.insert("title".to_owned(), Value::String(request.title.clone()));
    object.insert("digest".to_owned(), Value::String(request.digest.clone()));
    object.insert(
        "attempted_at".to_owned(),
        Value::Number(request.attempted_at.into()),
    );
    object.insert(
        "failure".to_owned(),
        request.failure.clone().map_or(Value::Null, Value::String),
    );
    if let Some(external_id) = external_id {
        object.insert(
            "external_id".to_owned(),
            Value::String(external_id.as_str().to_owned()),
        );
    }
    format!("{}\n", Value::Object(object))
}

/// Enumerates every outstanding marker under `<integrations>/<integration>/
/// pending-push/`.
///
/// # Errors
///
/// [`MarkerError::Io`] when the directory exists but cannot be read;
/// [`MarkerError::Malformed`] when a marker file cannot be parsed. A
/// missing directory yields an empty list, not an error.
pub fn outstanding(
    integrations_dir: &Path,
    integration: &str,
) -> Result<Vec<(PathBuf, PendingPush)>, MarkerError> {
    let dir = integrations_dir.join(integration).join("pending-push");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(MarkerError::Io(error.to_string())),
    };

    let mut markers = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| MarkerError::Io(error.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|error| MarkerError::Io(error.to_string()))?;
        if let Some(marker) = read(Some(&content))? {
            markers.push((path, marker));
        }
    }
    Ok(markers)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::outstanding;
    use super::path;
    use super::read;
    use super::render;
    use super::request_digest;
    use tracker::ExternalId;
    use work::sync::PendingPush;
    use work::sync::RequestFingerprint;

    fn fingerprint() -> RequestFingerprint {
        RequestFingerprint {
            title: "Fix flaky test".to_owned(),
            digest: request_digest("Fix flaky test", "Body\n", "story"),
            attempted_at: 1_700_000_000,
            failure: None,
        }
    }

    #[test]
    fn an_attempted_marker_round_trips() {
        let marker = PendingPush::Attempted {
            request: fingerprint(),
        };
        let rendered = render(&marker);
        let read_back = read(Some(&rendered))
            .expect("valid JSON parses")
            .expect("content is present");
        assert_eq!(read_back, marker);
    }

    #[test]
    fn a_created_marker_round_trips_including_the_external_id() {
        let marker = PendingPush::Created {
            request: fingerprint(),
            external_id: ExternalId::new("ENG-1".to_owned()),
        };
        let rendered = render(&marker);
        let read_back = read(Some(&rendered))
            .expect("valid JSON parses")
            .expect("content is present");
        assert_eq!(read_back, marker);
    }

    #[test]
    fn absent_content_reads_as_none() {
        assert!(read(None).expect("absent is not an error").is_none());
    }

    #[test]
    fn malformed_content_is_an_error_not_none() {
        assert!(read(Some("not json")).is_err());
    }

    #[test]
    fn the_digest_is_injective_over_field_boundaries() {
        let a = request_digest("ab", "c", "story");
        let b = request_digest("a", "bc", "story");
        assert_ne!(a, b);
    }

    #[test]
    fn path_inserts_the_pending_push_segment() {
        let resolved = path(
            std::path::Path::new("/repo/.accelerator/state/integrations"),
            "jira",
            "fix-flaky-test",
        );
        assert_eq!(
            resolved,
            std::path::Path::new(
                "/repo/.accelerator/state/integrations/jira/pending-push/fix-flaky-test.json"
            )
        );
    }

    #[test]
    fn outstanding_is_empty_for_a_missing_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let markers = outstanding(dir.path(), "jira").expect("no error");
        assert!(markers.is_empty());
    }

    #[test]
    fn outstanding_lists_every_marker_in_the_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pending_dir = dir.path().join("jira/pending-push");
        std::fs::create_dir_all(&pending_dir).expect("mkdir");
        let marker = PendingPush::Attempted {
            request: fingerprint(),
        };
        std::fs::write(pending_dir.join("one.json"), render(&marker))
            .expect("write");

        let markers = outstanding(dir.path(), "jira").expect("no error");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].1, marker);
    }
}
