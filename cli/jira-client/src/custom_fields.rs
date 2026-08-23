//! Resolving and coercing `--custom SLUG=VALUE` entries against the field
//! cache, the Rust twin of the retiring `jira-custom-fields.sh`.
//!
//! A token resolves to its `customfield_NNNNN` id through the `fields.json`
//! cache (matched by name, slug, id or key, in that order), and the raw value
//! is coerced by the field's `schema.type`. An `@json:` prefix bypasses
//! coercion, passing a validated JSON literal through verbatim — the only way to
//! set an array- or object-valued field.

use serde_json::Map;
use serde_json::Value;

/// Why a `--custom` entry could not be resolved or coerced. The binary maps
/// every arm to `CREATE_BAD_FIELD` / `UPDATE_BAD_FIELD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomFieldError {
    /// The entry carried no `=` separating slug from value.
    Malformed { entry: String },
    /// No cached field matches the token by name, slug, id or key.
    Unknown { token: String },
    /// The value is wrong for the field's schema type, or the type is one only
    /// an `@json:` literal can carry.
    BadValue { token: String, reason: String },
}

impl std::fmt::Display for CustomFieldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { entry } => {
                write!(formatter, "--custom {entry:?} is not SLUG=VALUE")
            }
            Self::Unknown { token } => write!(
                formatter,
                "no cached field matches {token:?}; run /init-jira \
                 --refresh-fields"
            ),
            Self::BadValue { token, reason } => {
                write!(formatter, "--custom {token}: {reason}")
            }
        }
    }
}

impl std::error::Error for CustomFieldError {}

/// Resolves and coerces every `SLUG=VALUE` entry into one `{field_id: value}`
/// map, merged in order.
///
/// `cache` is the parsed `fields.json` (`{fields: [{id, slug, name, key,
/// schema}]}`).
///
/// # Errors
///
/// [`CustomFieldError`] for an entry with no `=`, an unresolvable token, or a
/// value that does not fit the field's schema type.
pub fn coerce(
    cache: &Value,
    entries: &[String],
) -> Result<Map<String, Value>, CustomFieldError> {
    let fields = cache.get("fields").and_then(Value::as_array);
    let mut out = Map::new();
    for entry in entries {
        let (token, raw) = entry.split_once('=').ok_or_else(|| {
            CustomFieldError::Malformed {
                entry: entry.clone(),
            }
        })?;
        let field = fields
            .and_then(|fields| {
                fields.iter().find(|field| matches_token(field, token))
            })
            .ok_or_else(|| CustomFieldError::Unknown {
                token: token.to_owned(),
            })?;
        let id = field.get("id").and_then(Value::as_str).ok_or_else(|| {
            CustomFieldError::BadValue {
                token: token.to_owned(),
                reason: "the cached field carries no id".to_owned(),
            }
        })?;
        out.insert(id.to_owned(), coerce_value(field, token, raw)?);
    }
    Ok(out)
}

/// A field matches a token by name, slug, id or key.
fn matches_token(field: &Value, token: &str) -> bool {
    ["name", "slug", "id", "key"].iter().any(|attribute| {
        field.get(attribute).and_then(Value::as_str) == Some(token)
    })
}

fn coerce_value(
    field: &Value,
    token: &str,
    raw: &str,
) -> Result<Value, CustomFieldError> {
    if let Some(literal) = raw.strip_prefix("@json:") {
        return serde_json::from_str(literal).map_err(|error| {
            CustomFieldError::BadValue {
                token: token.to_owned(),
                reason: format!("@json: value is not valid JSON: {error}"),
            }
        });
    }
    match field.pointer("/schema/type").and_then(Value::as_str) {
        Some("number") => coerce_number(token, raw),
        Some("string" | "date" | "datetime") => {
            Ok(Value::String(raw.to_owned()))
        }
        Some("option") => Ok(serde_json::json!({ "value": raw })),
        Some("user") => Ok(serde_json::json!({ "accountId": raw })),
        Some(other) => Err(CustomFieldError::BadValue {
            token: token.to_owned(),
            reason: format!(
                "schema type {other:?} has no scalar coercion; pass \
                 @json:<literal>"
            ),
        }),
        None => Err(CustomFieldError::BadValue {
            token: token.to_owned(),
            reason: "the field has no schema type; pass @json:<literal>"
                .to_owned(),
        }),
    }
}

fn coerce_number(token: &str, raw: &str) -> Result<Value, CustomFieldError> {
    if !is_decimal(raw) {
        return Err(CustomFieldError::BadValue {
            token: token.to_owned(),
            reason: format!("{raw:?} is not a number"),
        });
    }
    serde_json::from_str(raw).map_err(|error| CustomFieldError::BadValue {
        token: token.to_owned(),
        reason: format!("{raw:?} is not a representable number: {error}"),
    })
}

/// The bash `^-?[0-9]+(\.[0-9]+)?$` shape: an optional sign, digits, and an
/// optional single fractional run. Rejects the scientific and hex forms a bare
/// serde parse would otherwise accept.
fn is_decimal(raw: &str) -> bool {
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    let (integer, fraction) = match digits.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (digits, None),
    };
    let is_run = |part: &str| {
        !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
    };
    is_run(integer) && fraction.is_none_or(is_run)
}
