//! The `summary` output: the plain text as assembled, or wrapped in the
//! `SessionStart` hook envelope.
//!
//! The envelope is built by hand, not `serde_json`, so the hexagon keeps its
//! narrow import surface; the JSON string escaping follows RFC 8259.

/// Wraps a summary in the compact `SessionStart` `additionalContext` envelope.
#[must_use]
pub fn hook_envelope(summary: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\
         \"additionalContext\":\"{}\"}}}}",
        json_escape(summary)
    )
}

fn json_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            control if (control as u32) < 0x20 => {
                let byte = control as u8;
                escaped.push_str("\\u00");
                escaped.push(
                    char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'),
                );
                escaped.push(
                    char::from_digit(u32::from(byte & 0xf), 16).unwrap_or('0'),
                );
            }
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{hook_envelope, json_escape};

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn the_envelope_carries_the_summary_as_additional_context() {
        let envelope = hook_envelope("line one\nline two");
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"line one\\nline two\"}}"
        );
    }
}
