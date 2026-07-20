//! The canonical-order JSONL record composer and the anchored remove prefix.
//! Both the writer and the remover route their opening bytes through one
//! `record_opener`, so they cannot drift on the load-bearing
//! `{"transformation_key":"<escaped>",` prefix.

use corpus::{Record, StoreError};

const RESERVED: [&str; 6] = [
    "transformation_key",
    "schema_version",
    "outcome",
    "proposed_value",
    "user_value",
    "timestamp",
];

fn escape_value(value: &str) -> Result<String, StoreError> {
    let quoted = serde_json::to_string(value).map_err(|error| {
        StoreError::Validation {
            detail: error.to_string(),
        }
    })?;
    Ok(quoted[1..quoted.len() - 1].to_owned())
}

fn is_valid_extras_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn require_non_empty(value: &str, field: &str) -> Result<(), StoreError> {
    if value.is_empty() {
        return Err(StoreError::Validation {
            detail: format!("{field} is required and must be non-empty"),
        });
    }
    Ok(())
}

/// The anchored opening bytes shared by the composer and the remover:
/// `{"transformation_key":"<escaped>",`. `transformation_key` is always followed
/// by `schema_version`, so the trailing comma is invariant.
fn record_opener(key: &str) -> Result<String, StoreError> {
    Ok(format!(
        "{{\"transformation_key\":\"{}\",",
        escape_value(key)?
    ))
}

fn push_string_field(
    out: &mut String,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    out.push_str(",\"");
    out.push_str(key);
    out.push_str("\":\"");
    out.push_str(&escape_value(value)?);
    out.push('"');
    Ok(())
}

pub fn compose_record(record: &Record) -> Result<String, StoreError> {
    require_non_empty(&record.transformation_key, "transformation_key")?;
    require_non_empty(&record.proposed_value, "proposed_value")?;
    require_non_empty(&record.timestamp, "timestamp")?;
    for (key, _) in &record.extras {
        if RESERVED.contains(&key.as_str()) {
            return Err(StoreError::Validation {
                detail: format!("reserved key '{key}' in extras position"),
            });
        }
        if !is_valid_extras_key(key) {
            return Err(StoreError::Validation {
                detail: format!("invalid extras key '{key}'"),
            });
        }
    }

    let mut out = record_opener(&record.transformation_key)?;
    out.push_str("\"schema_version\":");
    out.push_str(&record.schema_version.to_string());
    out.push_str(",\"outcome\":\"");
    out.push_str(record.outcome.as_str());
    out.push('"');
    push_string_field(&mut out, "proposed_value", &record.proposed_value)?;
    if let Some(user_value) = &record.user_value {
        push_string_field(&mut out, "user_value", user_value)?;
    }
    push_string_field(&mut out, "timestamp", &record.timestamp)?;
    for (key, value) in &record.extras {
        push_string_field(&mut out, key, value)?;
    }
    out.push('}');
    Ok(out)
}

pub fn remove_prefix(key: &str) -> Result<String, StoreError> {
    record_opener(key)
}

#[cfg(test)]
mod tests {
    use corpus::{Outcome, Record, StoreError};

    use super::{compose_record, remove_prefix};

    fn base() -> Record {
        Record {
            transformation_key: "greeting".to_owned(),
            schema_version: 1,
            outcome: Outcome::Edited,
            proposed_value: "hello".to_owned(),
            user_value: None,
            timestamp: "2026-07-19T00:00:00+00:00".to_owned(),
            extras: Vec::new(),
        }
    }

    #[test]
    fn the_canonical_order_is_pinned_without_a_user_value(
    ) -> Result<(), StoreError> {
        let mut record = base();
        record.extras = vec![("author".to_owned(), "toby".to_owned())];
        assert_eq!(
            compose_record(&record)?,
            "{\"transformation_key\":\"greeting\",\"schema_version\":1,\
             \"outcome\":\"edited\",\"proposed_value\":\"hello\",\
             \"timestamp\":\"2026-07-19T00:00:00+00:00\",\
             \"author\":\"toby\"}"
        );
        Ok(())
    }

    #[test]
    fn a_user_value_is_emitted_by_presence_not_outcome(
    ) -> Result<(), StoreError> {
        let mut record = base();
        record.outcome = Outcome::Accepted;
        record.user_value = Some("hi".to_owned());
        assert_eq!(
            compose_record(&record)?,
            "{\"transformation_key\":\"greeting\",\"schema_version\":1,\
             \"outcome\":\"accepted\",\"proposed_value\":\"hello\",\
             \"user_value\":\"hi\",\
             \"timestamp\":\"2026-07-19T00:00:00+00:00\"}"
        );
        Ok(())
    }

    #[test]
    fn an_empty_proposed_value_is_rejected() {
        let mut record = base();
        record.proposed_value = String::new();
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn an_empty_transformation_key_is_rejected() {
        let mut record = base();
        record.transformation_key = String::new();
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn an_empty_timestamp_is_rejected() {
        let mut record = base();
        record.timestamp = String::new();
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn a_reserved_extras_key_is_rejected() {
        let mut record = base();
        record.extras = vec![("outcome".to_owned(), "x".to_owned())];
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn a_malformed_extras_key_is_rejected() {
        let mut record = base();
        record.extras = vec![("Bad-Key".to_owned(), "x".to_owned())];
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn control_and_quote_and_backslash_escaping_is_pinned(
    ) -> Result<(), StoreError> {
        let mut record = base();
        record.transformation_key = "a\\b\"c\td\x7f".to_owned();
        let composed = compose_record(&record)?;
        assert!(
            composed.starts_with(
                "{\"transformation_key\":\"a\\\\b\\\"c\\td\x7f\","
            ),
            "escaping drifted: {composed}"
        );
        Ok(())
    }

    #[test]
    fn the_remove_prefix_matches_the_composed_opener() -> Result<(), StoreError>
    {
        let mut record = base();
        record.transformation_key = "a\\b\"c".to_owned();
        let composed = compose_record(&record)?;
        let prefix = remove_prefix("a\\b\"c")?;
        assert!(composed.starts_with(&prefix), "opener drift: {composed}");
        assert_eq!(prefix, "{\"transformation_key\":\"a\\\\b\\\"c\",");
        Ok(())
    }
}
