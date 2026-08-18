//! The content-type sniffer's table, including the unknown-type fallback: an
//! untested sniffer sends a wrong `contentType` and the attachment renders
//! wrong with nothing to catch it.

use tracker_support::mime::sniff;
use tracker_support::mime::OCTET_STREAM;

fn cases() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    vec![
        ("png", b"\x89PNG\r\n\x1a\n\x00\x00".to_vec(), "image/png"),
        (
            "jpeg",
            b"\xff\xd8\xff\xe0\x00\x10JFIF".to_vec(),
            "image/jpeg",
        ),
        ("gif87", b"GIF87a\x01\x00".to_vec(), "image/gif"),
        ("gif89", b"GIF89a\x01\x00".to_vec(), "image/gif"),
        ("pdf", b"%PDF-1.7\n".to_vec(), "application/pdf"),
        ("zip", b"PK\x03\x04\x14\x00".to_vec(), "application/zip"),
        (
            "zip-empty",
            b"PK\x05\x06\x00\x00".to_vec(),
            "application/zip",
        ),
        ("gzip", b"\x1f\x8b\x08\x00".to_vec(), "application/gzip"),
        ("ascii-text", b"hello, world\n".to_vec(), "text/plain"),
        ("utf8-text", "café ☕\n".as_bytes().to_vec(), "text/plain"),
        ("unknown-binary", vec![0x00, 0x01, 0x02, 0x00], OCTET_STREAM),
        ("empty", Vec::new(), OCTET_STREAM),
    ]
}

#[test]
fn every_case_sniffs_to_its_expected_type() {
    for (name, bytes, expected) in cases() {
        assert_eq!(sniff(&bytes), expected, "case {name}");
    }
}

#[test]
fn a_control_byte_in_otherwise_text_is_binary() {
    // A NUL is the classic binary tell; text detection must reject it rather
    // than send text/plain for a file that is not.
    assert_eq!(sniff(b"plain until \x00 here"), OCTET_STREAM);
}

#[test]
fn the_fallback_type_is_never_caller_controlled() {
    // The sniffer returns a &'static from a closed set, so no input value can
    // reach the returned type.
    assert_eq!(sniff(b"\x07\x08\x0b\x0c"), OCTET_STREAM);
}
