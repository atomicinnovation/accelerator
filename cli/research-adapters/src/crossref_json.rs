//! A Crossref work message, decoded into the domain's typed input.
//!
//! Only the type, container titles and publisher are read. Each is kept only
//! in the shape Crossref documents, so a null or unexpected value reads as
//! absent, but a body with no `message` object is undecodable.

use research::sources::crossref::CrossrefWork;
use research::sources::fetch::CrossrefDecoder;
use research::sources::fetch::DecodeFailure;
use serde::Deserialize;
use serde_json::Value;

pub struct JsonCrossrefDecoder;

impl CrossrefDecoder for JsonCrossrefDecoder {
    fn work(&self, body: &[u8]) -> Result<CrossrefWork, DecodeFailure> {
        let envelope: Envelope = serde_json::from_slice(body)
            .map_err(|_| DecodeFailure::Undecodable)?;
        let message = envelope.message;
        Ok(CrossrefWork {
            kind: text(message.get("type")),
            container_titles: message
                .get("container-title")
                .and_then(Value::as_array)
                .map(|titles| {
                    titles
                        .iter()
                        .filter_map(|title| text(Some(title)))
                        .collect()
                })
                .unwrap_or_default(),
            publisher: text(message.get("publisher")),
        })
    }
}

#[derive(Deserialize)]
struct Envelope {
    message: serde_json::Map<String, Value>,
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}
