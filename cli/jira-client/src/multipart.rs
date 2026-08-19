//! A hand-rolled `multipart/form-data` encoder for attachment uploads.
//!
//! `reqwest/multipart` is deliberately not enabled — it would pull `mime` and
//! `mime_guess` into the closure — so the injection safety it provides is
//! restated here as an explicit contract: the boundary is a CSPRNG token
//! verified absent from every part body, and a filename carrying a quote,
//! newline or control byte is refused rather than escaped, so no part header
//! can be smuggled through it.

use rand::Rng as _;

use crate::surface::SurfaceError;

/// The form field name, a fixed constant — never caller-supplied.
pub const PART_NAME: &str = "file";

/// How many boundary candidates to try before giving up. A collision with a
/// 128-bit random token is astronomically unlikely; the cap turns a
/// pathological input into a typed error rather than an infinite loop.
const BOUNDARY_ATTEMPTS: usize = 8;

/// One file to upload: its filename, sniffed content type, and bytes.
pub struct Part {
    pub filename: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Encodes the parts into a `multipart/form-data` body delimited by
/// `boundary`.
///
/// # Errors
///
/// [`SurfaceError::BadFilename`] for a filename that cannot appear in a
/// quoted-string `Content-Disposition`.
pub fn encode(boundary: &str, parts: &[Part]) -> Result<Vec<u8>, SurfaceError> {
    let mut body = Vec::new();
    for part in parts {
        refuse_unsafe_filename(&part.filename)?;
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{PART_NAME}\"; \
                 filename=\"{}\"\r\n",
                part.filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(
            format!("Content-Type: {}\r\n\r\n", part.content_type).as_bytes(),
        );
        body.extend_from_slice(&part.bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok(body)
}

/// Generates a boundary token guaranteed absent from every part body, so no
/// file's content can split or truncate the upload.
///
/// # Errors
///
/// [`SurfaceError::BoundaryExhausted`] if no free candidate is found — an
/// input engineered to collide with the CSPRNG on every attempt.
pub fn boundary_free_of(parts: &[Part]) -> Result<String, SurfaceError> {
    let mut rng = rand::rng();
    for _ in 0..BOUNDARY_ATTEMPTS {
        let candidate = format!(
            "----accelerator-{:016x}{:016x}",
            rng.random::<u64>(),
            rng.random::<u64>(),
        );
        let needle = candidate.as_bytes();
        if parts.iter().all(|part| !contains(&part.bytes, needle)) {
            return Ok(candidate);
        }
    }
    Err(SurfaceError::BoundaryExhausted)
}

fn refuse_unsafe_filename(name: &str) -> Result<(), SurfaceError> {
    let unsafe_byte = name.bytes().any(|byte| {
        byte == b'"'
            || byte == b'\r'
            || byte == b'\n'
            || byte < 0x20
            || byte == 0x7f
    });
    if unsafe_byte || name.is_empty() {
        return Err(SurfaceError::BadFilename {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
