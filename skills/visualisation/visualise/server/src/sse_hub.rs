use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::docs::DocTypeKey;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionKind {
    Created,
    Edited,
    Deleted,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SsePayload {
    DocChanged {
        action: ActionKind,
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        etag: Option<String>,
        timestamp: DateTime<Utc>,
    },
    DocInvalid {
        #[serde(rename = "docType")]
        doc_type: DocTypeKey,
        path: String,
    },
}

pub struct SseHub {
    tx: broadcast::Sender<SsePayload>,
}

impl SseHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SsePayload> {
        self.tx.subscribe()
    }

    pub fn broadcast(&self, payload: SsePayload) {
        let _ = self.tx.send(payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tokio::sync::broadcast::error::RecvError;

    fn make_event(path: &str) -> SsePayload {
        SsePayload::DocChanged {
            action: ActionKind::Edited,
            doc_type: crate::docs::DocTypeKey::Plans,
            path: path.to_string(),
            etag: Some("sha256-abc".to_string()),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn single_subscriber_receives_broadcast() {
        let hub = SseHub::new(16);
        let mut rx = hub.subscribe();
        hub.broadcast(make_event("meta/plans/foo.md"));
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, SsePayload::DocChanged { .. }));
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive_broadcast() {
        let hub = SseHub::new(16);
        let mut rx1 = hub.subscribe();
        let mut rx2 = hub.subscribe();
        hub.broadcast(make_event("meta/plans/foo.md"));
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[tokio::test]
    async fn send_with_no_subscribers_does_not_panic() {
        let hub = SseHub::new(16);
        hub.broadcast(make_event("meta/plans/foo.md"));
    }

    #[tokio::test]
    async fn slow_consumer_gets_lagged_error() {
        let hub = SseHub::new(2);
        let mut rx = hub.subscribe();
        for i in 0..10u32 {
            hub.broadcast(SsePayload::DocChanged {
                action: ActionKind::Edited,
                doc_type: crate::docs::DocTypeKey::Plans,
                path: format!("meta/plans/{i}.md"),
                etag: Some("sha256-x".into()),
                timestamp: Utc::now(),
            });
        }
        assert!(matches!(rx.recv().await, Err(RecvError::Lagged(_))));
        assert!(rx.recv().await.is_ok(), "expected recovery after lag");
    }

    #[test]
    fn sse_payload_json_wire_format() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();
        let changed = SsePayload::DocChanged {
            action: ActionKind::Edited,
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: Some("sha256-abc".into()),
            timestamp: ts,
        };
        let json = serde_json::to_string(&changed).unwrap();
        assert!(json.contains("\"type\":\"doc-changed\""), "json: {json}");
        assert!(json.contains("\"action\":\"edited\""), "json: {json}");
        assert!(json.contains("\"docType\":"), "json: {json}");
        assert!(json.contains("\"etag\":\"sha256-abc\""), "json: {json}");
        assert!(
            json.contains("\"timestamp\":\"2026-05-13T12:00:00Z\""),
            "json: {json}"
        );

        let deleted = SsePayload::DocChanged {
            action: ActionKind::Deleted,
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: None,
            timestamp: ts,
        };
        let json = serde_json::to_string(&deleted).unwrap();
        assert!(json.contains("\"action\":\"deleted\""), "json: {json}");
        assert!(
            !json.contains("etag"),
            "etag must be absent for deletions: {json}"
        );

        let invalid = SsePayload::DocInvalid {
            doc_type: crate::docs::DocTypeKey::Plans,
            path: "meta/plans/bad.md".into(),
        };
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(json.contains("\"type\":\"doc-invalid\""), "json: {json}");
    }
}
