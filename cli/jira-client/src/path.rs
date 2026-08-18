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
/// # Errors
///
/// [`ClientError::BadPath`] naming the condition that failed.
pub fn validate(path: &str) -> Result<(), ClientError> {
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

    let mut current = path.to_owned();
    for round in 1..=DECODE_ROUNDS {
        let decoded = decode_once(&current);
        if decoded == current {
            return Ok(());
        }
        if round == DECODE_ROUNDS {
            return Err(refuse("URL-decode iteration cap exceeded"));
        }
        if has_traversal(&decoded) {
            return Err(refuse("path traversal sequence"));
        }
        if decoded
            .chars()
            .any(|character| character.is_control() || character == '\u{7f}')
        {
            return Err(refuse("control character"));
        }
        current = decoded;
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
