//! Path validation, transcribed from `_req_validate_path`
//! (`jira-request.sh:69-145`) — the conditions behind bash code 17.
//!
//! Structure is checked on the encoded path; traversal is then checked again
//! after each decoding round, up to a cap of eight. The two layers are what
//! stop a double-encoded traversal from re-targeting an authenticated request
//! at another endpoint.

use crate::error::ClientError;

const PREFIX: &str = "/rest/api/3/";
const DECODE_ROUNDS: usize = 8;

const fn permitted(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '.' | '_' | '/' | '?' | '=' | '&' | ',' | ':' | '%' | '@' | '-'
        )
}

/// Accepts a request path safe to send under the caller's credentials.
///
/// For a path with no interpolated identifier. A composed path goes through
/// [`validate_composed`] instead, because decoding a percent-encoded segment
/// back into the whole path would reject a legitimate identifier containing
/// `/`.
///
/// # Errors
///
/// [`ClientError::BadPath`] naming the condition that failed.
pub fn validate(path: &str) -> Result<(), ClientError> {
    validate_structure(path)?;
    let refuse = |reason: &str| ClientError::BadPath {
        path: path.to_owned(),
        reason: reason.to_owned(),
    };

    let mut current = path.to_owned();
    for round in 1..=DECODE_ROUNDS {
        let decoded = decode_once(&current);
        if decoded == current {
            return Ok(());
        }
        if round == DECODE_ROUNDS {
            return Err(refuse("URL-decode iteration cap exceeded"));
        }
        refuse_traversal_or_control(&decoded, &refuse)?;
        current = decoded;
    }
    Ok(())
}

/// Validates a path assembled from a template plus percent-encoded segments.
///
/// Structure is checked on the **encoded** path and traversal on each
/// **decoded** segment in isolation. Flattening the two would contradict the
/// identifier rule, which permits `/` mid-token: a legitimate id containing one
/// encodes to `%2F`, and a decode-and-recheck pass over the composed path would
/// then see a traversal-shaped path and refuse an id the rule allows. The first
/// legitimate id to trip that reads as a bug, and the cheapest fix is to drop
/// one of the two layers.
///
/// # Errors
///
/// [`ClientError::BadPath`] naming the condition that failed.
pub fn validate_composed(
    path: &str,
    segments: &[&str],
) -> Result<(), ClientError> {
    validate_structure(path)?;
    for segment in segments {
        let refuse = |reason: &str| ClientError::BadPath {
            path: (*segment).to_owned(),
            reason: reason.to_owned(),
        };
        let mut current = (*segment).to_owned();
        refuse_traversal_or_control(&current, &refuse)?;
        for round in 1..=DECODE_ROUNDS {
            let decoded = decode_once(&current);
            if decoded == current {
                break;
            }
            if round == DECODE_ROUNDS {
                return Err(refuse("URL-decode iteration cap exceeded"));
            }
            refuse_traversal_or_control(&decoded, &refuse)?;
            current = decoded;
        }
    }
    Ok(())
}

/// Percent-encodes one path segment, so an identifier's `/`, `%`, `?`, `&` and
/// `=` cannot re-target the request at another endpoint.
#[must_use]
pub fn encode_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(char::from(byte)),
            other => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(other >> 4)]));
                encoded.push(char::from(HEX[usize::from(other & 0x0f)]));
            }
        }
    }
    encoded
}

fn refuse_traversal_or_control(
    candidate: &str,
    refuse: &impl Fn(&str) -> ClientError,
) -> Result<(), ClientError> {
    if has_traversal(candidate) {
        return Err(refuse("path traversal sequence"));
    }
    if candidate
        .chars()
        .any(|character| character.is_control() || character == '\u{7f}')
    {
        return Err(refuse("control character"));
    }
    Ok(())
}

/// The structural rules, on the encoded path.
fn validate_structure(path: &str) -> Result<(), ClientError> {
    let refuse = |reason: &str| ClientError::BadPath {
        path: path.to_owned(),
        reason: reason.to_owned(),
    };

    if !path.starts_with(PREFIX) {
        return Err(refuse("not under /rest/api/3/"));
    }
    if !path.chars().all(permitted) {
        return Err(refuse("contains disallowed characters"));
    }
    if has_traversal(path) {
        return Err(refuse("path traversal sequence"));
    }
    if path.contains("//") {
        return Err(refuse("consecutive slashes"));
    }
    Ok(())
}

fn has_traversal(path: &str) -> bool {
    path.split('/').any(|segment| segment == "..")
}

fn decode_once(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if let Some(byte) = escaped_byte(bytes, index) {
            decoded.push(byte);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn escaped_byte(bytes: &[u8], index: usize) -> Option<u8> {
    if *bytes.get(index)? != b'%' {
        return None;
    }
    let digit = |at: usize| char::from(*bytes.get(at)?).to_digit(16);
    let value = digit(index + 1)? * 16 + digit(index + 2)?;
    u8::try_from(value).ok()
}
