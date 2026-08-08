//! The canonical-order JSONL record composer, its parsing inverse, and the
//! anchored remove prefix.
//!
//! The writer and the remover route their opening bytes through one
//! `record_opener`, so they cannot drift on the load-bearing
//! `{"transformation_key":"<escaped>",` prefix.

use corpus::{Outcome, Record, StoreError};

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

/// `user_value` is presence-based, coupled one-for-one with `outcome`:
/// present if and only if the outcome is `Edited`. An accepted or skipped
/// record recording a user value (or an edited one recording none) is not a
/// state the interactive engine's own accept/edit/skip contract can produce
/// — composing it would silently launder a caller bug into a written record.
fn require_user_value_coupling(record: &Record) -> Result<(), StoreError> {
    match (record.outcome, &record.user_value) {
        (Outcome::Edited, None) => Err(StoreError::Validation {
            detail: "outcome is edited but user_value is absent".to_owned(),
        }),
        (Outcome::Accepted | Outcome::Skipped, Some(_)) => {
            Err(StoreError::Validation {
                detail: format!(
                    "outcome is {} but user_value is present",
                    record.outcome.as_str()
                ),
            })
        }
        _ => Ok(()),
    }
}

/// # Errors
/// [`StoreError::Validation`] when a required field is empty, `user_value`'s
/// presence disagrees with `outcome`, or an extras key is reserved or
/// malformed.
pub fn compose_record(record: &Record) -> Result<String, StoreError> {
    require_non_empty(&record.transformation_key, "transformation_key")?;
    require_non_empty(&record.proposed_value, "proposed_value")?;
    require_non_empty(&record.timestamp, "timestamp")?;
    require_user_value_coupling(record)?;
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

/// # Errors
/// [`StoreError::Validation`] when `key` cannot be escaped.
pub fn remove_prefix(key: &str) -> Result<String, StoreError> {
    record_opener(key)
}

fn invalid(detail: impl Into<String>) -> StoreError {
    StoreError::Validation {
        detail: detail.into(),
    }
}

fn required_str(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, StoreError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("missing or non-string '{field}'")))
}

/// The inverse of [`compose_record`].
///
/// Extras are recovered in the parsed JSON object's own key order — `serde_json`
/// without the `preserve_order` feature is `BTreeMap`-backed, so that order is
/// lexicographic by key, not necessarily the writer's original declaration
/// order. `compose_record` re-canonicalises deterministically regardless, so a
/// cutover through parse-then-compose is still idempotent and byte-stable; it
/// just does not reproduce a non-alphabetical extras order a hand-written or
/// bash-written record happened to use.
///
/// # Errors
/// [`StoreError::Validation`] when the line is not valid JSON, is not an
/// object, is missing a required field, or its `outcome` is not one of
/// `accepted`/`edited`/`skipped`.
pub fn parse_record(line: &str) -> Result<Record, StoreError> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| invalid(error.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid("record is not a JSON object"))?;

    let transformation_key = required_str(object, "transformation_key")?;
    let schema_version = object
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| invalid("missing or non-numeric 'schema_version'"))?;
    let outcome = match required_str(object, "outcome")?.as_str() {
        "accepted" => Outcome::Accepted,
        "edited" => Outcome::Edited,
        "skipped" => Outcome::Skipped,
        other => return Err(invalid(format!("unknown outcome '{other}'"))),
    };
    let proposed_value = required_str(object, "proposed_value")?;
    let user_value = object
        .get("user_value")
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("non-string 'user_value'"))
        })
        .transpose()?;
    let timestamp = required_str(object, "timestamp")?;

    let mut extras = Vec::new();
    for (key, value) in object {
        if RESERVED.contains(&key.as_str()) {
            continue;
        }
        let value = value.as_str().map(str::to_owned).ok_or_else(|| {
            invalid(format!("non-string extras field '{key}'"))
        })?;
        extras.push((key.clone(), value));
    }

    Ok(Record {
        transformation_key,
        schema_version,
        outcome,
        proposed_value,
        user_value,
        timestamp,
        extras,
    })
}

#[cfg(test)]
mod tests {
    use corpus::{Outcome, Record, StoreError};

    use super::{compose_record, parse_record, remove_prefix};

    fn base() -> Record {
        Record {
            transformation_key: "greeting".to_owned(),
            schema_version: 1,
            outcome: Outcome::Accepted,
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
             \"outcome\":\"accepted\",\"proposed_value\":\"hello\",\
             \"timestamp\":\"2026-07-19T00:00:00+00:00\",\
             \"author\":\"toby\"}"
        );
        Ok(())
    }

    #[test]
    fn a_user_value_is_emitted_when_present() -> Result<(), StoreError> {
        let mut record = base();
        record.outcome = Outcome::Edited;
        record.user_value = Some("hi".to_owned());
        assert_eq!(
            compose_record(&record)?,
            "{\"transformation_key\":\"greeting\",\"schema_version\":1,\
             \"outcome\":\"edited\",\"proposed_value\":\"hello\",\
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

    #[test]
    fn compose_then_parse_round_trips_a_plain_record() -> Result<(), StoreError>
    {
        let record = base();
        let parsed = parse_record(&compose_record(&record)?)?;
        assert_eq!(parsed, record);
        Ok(())
    }

    #[test]
    fn compose_then_parse_round_trips_a_user_value() -> Result<(), StoreError> {
        let mut record = base();
        record.outcome = Outcome::Edited;
        record.user_value = Some("hi".to_owned());
        let parsed = parse_record(&compose_record(&record)?)?;
        assert_eq!(parsed, record);
        Ok(())
    }

    #[test]
    fn compose_then_parse_round_trips_skipped() -> Result<(), StoreError> {
        let mut record = base();
        record.outcome = Outcome::Skipped;
        let parsed = parse_record(&compose_record(&record)?)?;
        assert_eq!(parsed, record);
        Ok(())
    }

    #[test]
    fn compose_then_parse_round_trips_adversarial_content(
    ) -> Result<(), StoreError> {
        let mut record = base();
        record.transformation_key =
            "quote\"backslash\\tab\ttab2\newline\nnon-ascii-\u{00e9}\u{4e2d}"
                .to_owned();
        record.proposed_value = record.transformation_key.clone();
        let parsed = parse_record(&compose_record(&record)?)?;
        assert_eq!(parsed, record);
        Ok(())
    }

    #[test]
    fn compose_then_parse_round_trips_extras_by_set() -> Result<(), StoreError>
    {
        let mut record = base();
        record.extras = vec![
            ("author".to_owned(), "toby".to_owned()),
            ("zed".to_owned(), "last".to_owned()),
        ];
        let parsed = parse_record(&compose_record(&record)?)?;
        let mut expected = record.extras;
        let mut actual = parsed.extras;
        expected.sort();
        actual.sort();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn parse_record_rejects_malformed_json() {
        assert!(matches!(
            parse_record("not json"),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn parse_record_rejects_a_missing_required_field() {
        assert!(matches!(
            parse_record("{\"transformation_key\":\"x\"}"),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn parse_record_rejects_an_unknown_outcome() {
        let line = "{\"transformation_key\":\"x\",\"schema_version\":1,\
                     \"outcome\":\"maybe\",\"proposed_value\":\"y\",\
                     \"timestamp\":\"2026-07-19T00:00:00+00:00\"}";
        assert!(matches!(
            parse_record(line),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn an_edited_record_with_no_user_value_is_rejected() {
        let mut record = base();
        record.outcome = Outcome::Edited;
        record.user_value = None;
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn an_accepted_record_with_a_user_value_is_rejected() {
        let mut record = base();
        record.outcome = Outcome::Accepted;
        record.user_value = Some("hi".to_owned());
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }

    #[test]
    fn a_skipped_record_with_a_user_value_is_rejected() {
        let mut record = base();
        record.outcome = Outcome::Skipped;
        record.user_value = Some("hi".to_owned());
        assert!(matches!(
            compose_record(&record),
            Err(StoreError::Validation { .. })
        ));
    }
}
