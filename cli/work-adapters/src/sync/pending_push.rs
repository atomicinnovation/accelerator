//! The pending-push marker's JSON encoding and filesystem path — the
//! adapter half of `work::sync::push_precondition`'s domain types.

use std::path::Path;
use std::path::PathBuf;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use sha2::Digest as _;
use sha2::Sha256;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use work::draft_id::DraftId;
use work::promotion::without_ids;
use work::promotion::IntendedBaseline;
use work::promotion::PromotionRecord;
use work::promotion::PromotionStage;
use work::promotion::ReadBack;
use work::promotion::RemoteHash;
use work::promotion::RemoteKeptReason;
use work::sync::PendingPush;
use work::sync::RequestFingerprint;

const PROMOTION_SCHEMA: u64 = 2;

/// A file in the pending-push directory: a legacy marker a create names by
/// its slug or item id, or a promotion record named by its draft ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    Legacy(PendingPush),
    Promotion(PromotionRecord),
}

/// One pending-push file: readable, with its path, or reported unreadable.
pub type MarkerEntry = Result<(PathBuf, Marker), UnreadableMarker>;

/// A marker file that exists but could not be read or understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableMarker {
    pub path: PathBuf,
    pub detail: String,
}

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

/// Prepares the pending-push directory, fail-closed.
///
/// Creates the directory and writes a directory-local `.gitignore` of `*`,
/// verifying it before any marker is written, so a run that cannot prove its
/// markers ignored writes none. Returns the prepared directory.
///
/// # Errors
///
/// The underlying I/O error when the directory or its `.gitignore` cannot be
/// created, or when the written ignore cannot be read back and verified.
pub fn prepare_dir(
    integrations_dir: &Path,
    integration: &str,
) -> std::io::Result<PathBuf> {
    let dir = integrations_dir.join(integration).join("pending-push");
    std::fs::create_dir_all(&dir)?;
    let gitignore = dir.join(".gitignore");
    std::fs::write(&gitignore, "*\n")?;
    if !std::fs::read_to_string(&gitignore)?.contains('*') {
        return Err(std::io::Error::other(
            "the pending-push .gitignore could not be verified",
        ));
    }
    Ok(dir)
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

/// The request digest of `body` with `id` normalised back to the template
/// placeholder, so two requests for one content digest alike whatever ID
/// each was given.
#[must_use]
pub fn content_digest(title: &str, body: &str, kind: &str, id: &str) -> String {
    request_digest(title, &without_ids(body, &[id]), kind)
}

/// `<integrations>/<integration>/pending-push/<draft-id>.json`.
#[must_use]
pub fn record_path(
    integrations_dir: &Path,
    integration: &str,
    draft: &DraftId,
) -> PathBuf {
    path(integrations_dir, integration, draft.as_str())
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

fn text(
    object: &Map<String, Value>,
    field: &str,
) -> Result<String, MarkerError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| MarkerError::Malformed(format!("missing '{field}'")))
}

fn optional_text(object: &Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(Value::as_str).map(str::to_owned)
}

fn key_of(object: &Map<String, Value>) -> Result<ExternalId, MarkerError> {
    text(object, "external_id").map(ExternalId::new)
}

/// A read-back as `<prefix>_hash` and `<prefix>_updated_at`, the stamp
/// persisted as the sync baseline persists it: reported, or null.
fn read_back_from(
    object: &Map<String, Value>,
    prefix: &str,
) -> Result<ReadBack, MarkerError> {
    Ok(ReadBack {
        hash: text(object, &format!("{prefix}_hash"))?,
        updated: optional_text(object, &format!("{prefix}_updated_at"))
            .map_or(RemoteTimestamp::NotRead, RemoteTimestamp::Reported),
    })
}

fn put_read_back(
    object: &mut Map<String, Value>,
    prefix: &str,
    read_back: &ReadBack,
) {
    object.insert(format!("{prefix}_hash"), json!(read_back.hash));
    object.insert(
        format!("{prefix}_updated_at"),
        json!(read_back.updated.reported()),
    );
}

fn kept_reason(
    object: &Map<String, Value>,
) -> Result<RemoteKeptReason, MarkerError> {
    let read_back = read_back_from(object, "read_back")?;
    match text(object, "reason")?.as_str() {
        "edited" => Ok(RemoteKeptReason::Edited { read_back }),
        "user-named-adopt" => {
            Ok(RemoteKeptReason::UserNamedAdopt { read_back })
        }
        "update-failed" => Ok(RemoteKeptReason::UpdateFailed { read_back }),
        "no-hash" => Ok(RemoteKeptReason::NoHash { read_back }),
        other => Err(MarkerError::Malformed(format!(
            "unrecognised remote-kept reason: {other}"
        ))),
    }
}

const fn reason_keyword(reason: &RemoteKeptReason) -> &'static str {
    match reason {
        RemoteKeptReason::Edited { .. } => "edited",
        RemoteKeptReason::UserNamedAdopt { .. } => "user-named-adopt",
        RemoteKeptReason::UpdateFailed { .. } => "update-failed",
        RemoteKeptReason::NoHash { .. } => "no-hash",
    }
}

fn baseline_from(
    value: Option<&Value>,
) -> Result<IntendedBaseline, MarkerError> {
    let object = value.and_then(Value::as_object).ok_or_else(|| {
        MarkerError::Malformed("missing 'baseline'".to_owned())
    })?;
    let remote_hash = if object.get("remote_hash").is_some_and(Value::is_null) {
        RemoteHash::Unknown
    } else {
        RemoteHash::Known(read_back_from(object, "remote")?)
    };
    Ok(IntendedBaseline {
        remote_hash,
        local_hash: text(object, "local_hash")?,
    })
}

fn stage_from(
    object: &Map<String, Value>,
) -> Result<PromotionStage, MarkerError> {
    match text(object, "kind")?.as_str() {
        "attempted" => Ok(PromotionStage::Attempted),
        "created" => Ok(PromotionStage::Created {
            key: key_of(object)?,
            created_remote_hash: optional_text(object, "created_remote_hash"),
        }),
        "remote-retitled" => Ok(PromotionStage::RemoteRetitled {
            key: key_of(object)?,
            read_back: read_back_from(object, "remote")?,
        }),
        "remote-kept" => Ok(PromotionStage::RemoteKept {
            key: key_of(object)?,
            reason: kept_reason(object)?,
        }),
        "retiring" => {
            let before =
                object.get("before").and_then(Value::as_object).ok_or_else(
                    || MarkerError::Malformed("missing 'before'".to_owned()),
                )?;
            Ok(PromotionStage::Retiring {
                key: key_of(object)?,
                baseline: baseline_from(object.get("baseline"))?,
                recovery_dir: PathBuf::from(text(object, "recovery_dir")?),
                before: Box::new(stage_from(before)?),
            })
        }
        other => Err(MarkerError::Malformed(format!(
            "unrecognised promotion stage: {other}"
        ))),
    }
}

fn stage_fields(stage: &PromotionStage) -> Map<String, Value> {
    let mut object = Map::new();
    let mut put = |field: &str, value: Value| {
        object.insert(field.to_owned(), value);
    };
    let key = |key: &ExternalId| Value::String(key.as_str().to_owned());
    match stage {
        PromotionStage::Attempted => put("kind", json!("attempted")),
        PromotionStage::Created {
            key: created,
            created_remote_hash,
        } => {
            put("kind", json!("created"));
            put("external_id", key(created));
            put("created_remote_hash", json!(created_remote_hash));
        }
        PromotionStage::RemoteRetitled {
            key: retitled,
            read_back,
        } => {
            put("kind", json!("remote-retitled"));
            put("external_id", key(retitled));
            put_read_back(&mut object, "remote", read_back);
        }
        PromotionStage::RemoteKept { key: kept, reason } => {
            put("kind", json!("remote-kept"));
            put("external_id", key(kept));
            put("reason", json!(reason_keyword(reason)));
            put_read_back(&mut object, "read_back", reason.read_back());
        }
        PromotionStage::Retiring {
            key: retiring,
            baseline,
            recovery_dir,
            before,
        } => {
            put("kind", json!("retiring"));
            put("external_id", key(retiring));
            let mut encoded = Map::new();
            match &baseline.remote_hash {
                RemoteHash::Known(read_back) => {
                    put_read_back(&mut encoded, "remote", read_back);
                }
                RemoteHash::Unknown => {
                    encoded.insert("remote_hash".to_owned(), Value::Null);
                }
            }
            encoded.insert("local_hash".to_owned(), json!(baseline.local_hash));
            put("baseline", Value::Object(encoded));
            put("recovery_dir", json!(recovery_dir.display().to_string()));
            put("before", Value::Object(stage_fields(before)));
        }
    }
    object
}

/// Renders a promotion record. Its `Attempted` and `Created` stages keep a
/// legacy marker's `kind` and fields, so an older binary still reads them.
#[must_use]
pub fn render_record(record: &PromotionRecord) -> String {
    let mut object = stage_fields(&record.stage);
    let request = &record.request;
    object.insert("schema".to_owned(), json!(PROMOTION_SCHEMA));
    object.insert("draft_id".to_owned(), json!(record.draft_id.as_str()));
    object.insert("content_digest".to_owned(), json!(record.content_digest));
    object.insert("title".to_owned(), json!(request.title));
    object.insert("digest".to_owned(), json!(request.digest));
    object.insert("attempted_at".to_owned(), json!(request.attempted_at));
    object.insert("failure".to_owned(), json!(request.failure));
    format!("{}\n", Value::Object(object))
}

fn record_from(
    object: &Map<String, Value>,
) -> Result<PromotionRecord, MarkerError> {
    let draft = text(object, "draft_id")?;
    Ok(PromotionRecord {
        draft_id: DraftId::parse(&draft).ok_or_else(|| {
            MarkerError::Malformed(format!("not a draft ID: {draft}"))
        })?,
        request: fingerprint_from(object).ok_or_else(|| {
            MarkerError::Malformed(
                "missing or malformed fingerprint fields".to_owned(),
            )
        })?,
        content_digest: text(object, "content_digest")?,
        stage: stage_from(object)?,
    })
}

/// Parses any pending-push file: a promotion record when it carries its
/// schema, a legacy marker otherwise.
///
/// # Errors
///
/// [`MarkerError::Malformed`] when `content` is present but neither.
pub fn read_marker(
    content: Option<&str>,
) -> Result<Option<Marker>, MarkerError> {
    let Some(raw) = content else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| MarkerError::Malformed(error.to_string()))?;
    let object = value.as_object().ok_or_else(|| {
        MarkerError::Malformed("not a JSON object".to_owned())
    })?;
    match object.get("schema").map(Value::as_u64) {
        None => read(Some(raw)).map(|marker| marker.map(Marker::Legacy)),
        Some(Some(PROMOTION_SCHEMA)) => {
            record_from(object).map(|record| Some(Marker::Promotion(record)))
        }
        Some(_) => Err(MarkerError::Malformed(
            "unsupported pending-push schema".to_owned(),
        )),
    }
}

/// Enumerates every marker under `<integrations>/<integration>/
/// pending-push/`, each one readable or reported unreadable, so one bad
/// file hides none of the others.
///
/// # Errors
///
/// [`MarkerError::Io`] when the directory exists but cannot be listed. A
/// missing directory yields an empty list.
pub fn outstanding(
    integrations_dir: &Path,
    integration: &str,
) -> Result<Vec<MarkerEntry>, MarkerError> {
    let dir = integrations_dir.join(integration).join("pending-push");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => return Err(MarkerError::Io(error.to_string())),
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(std::ffi::OsStr::to_str) == Some("json")
        })
        .collect();
    paths.sort();
    Ok(paths
        .into_iter()
        .filter_map(|path| {
            let unreadable = |detail: String| UnreadableMarker {
                path: path.clone(),
                detail,
            };
            let read = std::fs::read_to_string(&path)
                .map_err(|error| unreadable(error.to_string()))
                .and_then(|content| {
                    read_marker(Some(&content))
                        .map_err(|error| unreadable(error.to_string()))
                });
            match read {
                Ok(Some(marker)) => Some(Ok((path.clone(), marker))),
                Ok(None) => None,
                Err(unreadable) => Some(Err(unreadable)),
            }
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use super::content_digest;
    use super::outstanding;
    use super::path;
    use super::prepare_dir;
    use super::read;
    use super::read_marker;
    use super::render;
    use super::render_record;
    use super::request_digest;
    use super::Marker;
    use tracker::ExternalId;
    use tracker::RemoteTimestamp;
    use work::draft_id::DraftId;
    use work::promotion::IntendedBaseline;
    use work::promotion::PromotionRecord;
    use work::promotion::PromotionStage;
    use work::promotion::ReadBack;
    use work::promotion::RemoteHash;
    use work::promotion::RemoteKeptReason;
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
    fn prepare_dir_writes_a_config_independent_ignore() {
        let root = tempfile::tempdir().expect("tempdir");
        let dir = prepare_dir(root.path(), "jira").expect("prepared");

        let ignore = std::fs::read_to_string(dir.join(".gitignore"))
            .expect("the directory-local .gitignore is written");
        assert!(ignore.contains('*'), "{ignore}");
        assert_eq!(dir, root.path().join("jira").join("pending-push"));
    }

    #[test]
    fn prepare_dir_fails_closed_when_the_ignore_cannot_be_written() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = tempfile::tempdir().expect("tempdir");
        let pending = root.path().join("jira").join("pending-push");
        std::fs::create_dir_all(&pending).expect("mkdir");
        std::fs::set_permissions(
            &pending,
            std::fs::Permissions::from_mode(0o555),
        )
        .expect("chmod");

        let result = prepare_dir(root.path(), "jira");

        let restore = std::fs::set_permissions(
            &pending,
            std::fs::Permissions::from_mode(0o755),
        );
        restore.expect("restore perms");
        assert!(result.is_err(), "a read-only directory must fail closed");
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
        assert_eq!(
            markers[0].as_ref().expect("readable").1,
            Marker::Legacy(marker)
        );
    }

    fn key() -> ExternalId {
        ExternalId::new("PP-900".to_owned())
    }

    fn promotion(stage: PromotionStage) -> PromotionRecord {
        PromotionRecord {
            draft_id: DraftId::parse("draft-k7mq3x").expect("draft id"),
            request: fingerprint(),
            content_digest: content_digest(
                "Fix flaky test",
                "# draft-k7mq3x: Fix flaky test\n",
                "story",
                "draft-k7mq3x",
            ),
            stage,
        }
    }

    fn every_stage() -> Vec<PromotionStage> {
        let retitled = PromotionStage::RemoteRetitled {
            key: key(),
            read_back: ReadBack {
                hash: "after".to_owned(),
                updated: RemoteTimestamp::Reported("2026-09-28".to_owned()),
            },
        };
        let kept = |reason| PromotionStage::RemoteKept { key: key(), reason };
        let read_back_hash = || ReadBack {
            hash: "r".to_owned(),
            updated: RemoteTimestamp::NotRead,
        };
        vec![
            PromotionStage::Attempted,
            PromotionStage::Created {
                key: key(),
                created_remote_hash: Some("created".to_owned()),
            },
            PromotionStage::Created {
                key: key(),
                created_remote_hash: None,
            },
            retitled.clone(),
            kept(RemoteKeptReason::Edited {
                read_back: read_back_hash(),
            }),
            kept(RemoteKeptReason::UserNamedAdopt {
                read_back: read_back_hash(),
            }),
            kept(RemoteKeptReason::UpdateFailed {
                read_back: read_back_hash(),
            }),
            kept(RemoteKeptReason::NoHash {
                read_back: read_back_hash(),
            }),
            PromotionStage::Retiring {
                key: key(),
                baseline: IntendedBaseline {
                    remote_hash: RemoteHash::Known(ReadBack {
                        hash: "after".to_owned(),
                        updated: RemoteTimestamp::Reported("t".to_owned()),
                    }),
                    local_hash: "promoted".to_owned(),
                },
                recovery_dir: PathBuf::from(
                    "retirement-recovery/draft-k7mq3x--PP-900",
                ),
                before: Box::new(retitled),
            },
            PromotionStage::Retiring {
                key: key(),
                baseline: IntendedBaseline {
                    remote_hash: RemoteHash::Unknown,
                    local_hash: "draft".to_owned(),
                },
                recovery_dir: PathBuf::from("x"),
                before: Box::new(kept(RemoteKeptReason::Edited {
                    read_back: read_back_hash(),
                })),
            },
        ]
    }

    #[test]
    fn a_created_marker_without_a_remote_hash_still_reads() {
        let record = promotion(PromotionStage::Created {
            key: key(),
            created_remote_hash: None,
        });
        let rendered = render_record(&record);
        assert!(
            rendered.contains("\"created_remote_hash\":null"),
            "{rendered}"
        );
        assert_eq!(
            read_marker(Some(&rendered)).expect("reads"),
            Some(Marker::Promotion(record))
        );
    }

    #[test]
    fn the_promotion_record_round_trips_at_every_stage() {
        for stage in every_stage() {
            let record = promotion(stage);
            let rendered = render_record(&record);
            assert!(rendered.contains("\"schema\":2"), "{rendered}");
            assert_eq!(
                read_marker(Some(&rendered)).expect("reads"),
                Some(Marker::Promotion(record)),
                "{rendered}"
            );
        }
    }

    #[test]
    fn the_content_digest_is_independent_of_the_substituted_id() {
        let digest = |id: &str| {
            content_digest(
                "T",
                &format!("# {id}: T\n\nSee {id}.\n"),
                "task",
                id,
            )
        };
        assert_eq!(digest("draft-k7mq3x"), digest("draft-p2r9zz"));
        assert_ne!(
            digest("draft-k7mq3x"),
            content_digest(
                "T",
                "# draft-k7mq3x: T\n\nOther.\n",
                "task",
                "draft-k7mq3x"
            )
        );
    }

    #[test]
    fn request_digest_is_unchanged_so_a_legacy_created_marker_still_reuses_its_id(
    ) {
        assert_eq!(
            request_digest("Fix flaky test", "Body\n", "story"),
            "4d280d2cb8ceccb4c38794a92dca8ef9d03a9560bf7573282a6d086d09a0aa47"
        );
    }

    #[test]
    fn a_legacy_marker_and_every_promotion_stage_coexist_in_one_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pending = prepare_dir(dir.path(), "linear").expect("prepared");
        let legacy = PendingPush::Attempted {
            request: fingerprint(),
        };
        std::fs::write(pending.join("fix-flaky-test.json"), render(&legacy))
            .expect("write legacy");
        let records: Vec<PromotionRecord> =
            every_stage().into_iter().map(promotion).collect();
        for (index, record) in records.iter().enumerate() {
            std::fs::write(
                pending.join(format!("record-{index:02}.json")),
                render_record(record),
            )
            .expect("write record");
        }

        let listed: Vec<Marker> = outstanding(dir.path(), "linear")
            .expect("listed")
            .into_iter()
            .map(|entry| entry.expect("readable").1)
            .collect();

        assert!(listed.contains(&Marker::Legacy(legacy)));
        for record in records {
            assert!(listed.contains(&Marker::Promotion(record)));
        }
    }

    #[test]
    fn an_unreadable_marker_is_reported_and_the_others_still_enumerate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pending = prepare_dir(dir.path(), "linear").expect("prepared");
        std::fs::write(pending.join("draft-k7mq3x.json"), "{").expect("torn");
        let record = promotion(PromotionStage::Attempted);
        std::fs::write(
            pending.join("draft-p2r9zz.json"),
            render_record(&record),
        )
        .expect("write");

        let listed = outstanding(dir.path(), "linear").expect("listed");

        assert_eq!(listed.len(), 2);
        let unreadable = listed[0].as_ref().expect_err("the torn file");
        assert_eq!(unreadable.path, pending.join("draft-k7mq3x.json"));
        assert_eq!(
            listed[1].as_ref().expect("readable").1,
            Marker::Promotion(record)
        );
    }

    #[test]
    fn attempted_and_created_promotion_records_read_as_legacy_markers_to_the_old_reader(
    ) {
        let attempted = render_record(&promotion(PromotionStage::Attempted));
        assert_eq!(
            read(Some(&attempted)).expect("reads"),
            Some(PendingPush::Attempted {
                request: fingerprint()
            })
        );
        let created = render_record(&promotion(PromotionStage::Created {
            key: key(),
            created_remote_hash: Some("h".to_owned()),
        }));
        assert_eq!(
            read(Some(&created)).expect("reads"),
            Some(PendingPush::Created {
                request: fingerprint(),
                external_id: key(),
            })
        );
    }
}
