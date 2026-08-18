//! A hand-rolled content-type sniffer, shared by both providers' attachment
//! uploads.
//!
//! The bash flows shell out to `file -b --mime-type` (and `curl` sniffs by
//! extension); reproducing that with a dependency would enlarge the licence
//! closure for a handful of magic numbers. The returned type is always from a
//! **closed set** — never echoed from caller input — so a hostile filename or
//! body cannot dictate the `Content-Type` a request carries.

/// The type returned when no signature matches and the sample is not text.
pub const OCTET_STREAM: &str = "application/octet-stream";

/// The number of leading bytes a text check inspects. A file larger than this
/// is judged text on its opening bytes, as `file` judges on a bounded sample.
const TEXT_SAMPLE: usize = 8192;

/// Infers a content type from a file's leading bytes.
///
/// Magic-number signatures win first; failing those, a sample that is valid
/// UTF-8 carrying no NUL and no other C0 control beyond tab/newline/carriage
/// return is `text/plain`; everything else is [`OCTET_STREAM`].
#[must_use]
pub fn sniff(sample: &[u8]) -> &'static str {
    if let Some(kind) = signature(sample) {
        return kind;
    }
    if looks_like_text(sample) {
        return "text/plain";
    }
    OCTET_STREAM
}

fn signature(sample: &[u8]) -> Option<&'static str> {
    const SIGNATURES: &[(&[u8], &str)] = &[
        (b"\x89PNG\r\n\x1a\n", "image/png"),
        (b"\xff\xd8\xff", "image/jpeg"),
        (b"GIF87a", "image/gif"),
        (b"GIF89a", "image/gif"),
        (b"%PDF-", "application/pdf"),
        (b"PK\x03\x04", "application/zip"),
        (b"PK\x05\x06", "application/zip"),
        (b"PK\x07\x08", "application/zip"),
        (b"\x1f\x8b", "application/gzip"),
    ];
    SIGNATURES
        .iter()
        .find(|(magic, _)| sample.starts_with(magic))
        .map(|(_, kind)| *kind)
}

fn looks_like_text(sample: &[u8]) -> bool {
    if sample.is_empty() {
        return false;
    }
    let head = &sample[..sample.len().min(TEXT_SAMPLE)];
    if std::str::from_utf8(head).is_err() {
        // A truncated multi-byte sequence at the sample boundary is not
        // evidence of binary, so a valid prefix is enough.
        let valid = utf8_valid_prefix(head);
        if valid == 0 {
            return false;
        }
        return head[..valid].iter().all(|&byte| text_byte(byte));
    }
    head.iter().all(|&byte| text_byte(byte))
}

const fn utf8_valid_prefix(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Ok(_) => bytes.len(),
        Err(error) => error.valid_up_to(),
    }
}

const fn text_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r') || byte >= 0x20
}
