//! JSON envelope shapes emitted by accelerator's Claude Code hooks.
//!
//! Built by hand rather than via `serde_json`, so callers keep a narrow
//! import surface; the JSON string escaping follows RFC 8259.

/// Wraps `context` in the `SessionStart` `additionalContext` envelope,
/// optionally carrying a top-level `systemMessage` alongside it.
#[must_use]
pub fn session_start(context: &str, system_message: Option<&str>) -> String {
    match system_message {
        Some(message) if !message.is_empty() => format!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"{}\"}},\"systemMessage\":\"{}\"}}",
            json_escape(context),
            json_escape(message)
        ),
        _ => format!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"{}\"}}}}",
            json_escape(context)
        ),
    }
}

/// A `PreToolUse` denial: blocks the tool call with `reason`.
#[must_use]
pub fn pre_tool_use_deny(reason: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\
         \"permissionDecisionReason\":\"{}\"}}}}",
        json_escape(reason)
    )
}

/// A `PreToolUse` warning that lets the tool call through.
///
/// A bare top-level `systemMessage`, with no `hookSpecificOutput` and no
/// `permissionDecision` key, so it falls through to the normal permission
/// flow rather than being read as a decision.
#[must_use]
pub fn pre_tool_use_warn(system_message: &str) -> String {
    bare_system_message(system_message)
}

/// Signals that an adapter failed to determine the facts a hook needs.
///
/// The hook degrades to a warning rather than blocking on unreliable
/// information. Same wire shape as [`pre_tool_use_warn`]; kept as a
/// distinct function because the two represent different call sites.
#[must_use]
pub fn adapter_failure(system_message: &str) -> String {
    bare_system_message(system_message)
}

fn bare_system_message(system_message: &str) -> String {
    format!("{{\"systemMessage\":\"{}\"}}", json_escape(system_message))
}

pub(crate) fn json_escape(text: &str) -> String {
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
    use super::{
        adapter_failure, json_escape, pre_tool_use_deny, pre_tool_use_warn,
        session_start,
    };

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn session_start_without_a_system_message_carries_only_the_context() {
        let envelope = session_start("line one\nline two", None);
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"line one\\nline two\"}}"
        );
    }

    #[test]
    fn session_start_with_an_empty_system_message_carries_only_the_context() {
        let envelope = session_start("context", Some(""));
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"context\"}}"
        );
    }

    #[test]
    fn session_start_with_a_system_message_carries_both() {
        let envelope = session_start("context", Some("jj binary missing"));
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"context\"},\
             \"systemMessage\":\"jj binary missing\"}"
        );
    }

    #[test]
    fn pre_tool_use_deny_carries_the_permission_decision() {
        let envelope = pre_tool_use_deny("Use jj instead of git status.");
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\
             \"permissionDecisionReason\":\"Use jj instead of git status.\"}}"
        );
    }

    #[test]
    fn pre_tool_use_warn_is_a_bare_system_message() {
        let envelope = pre_tool_use_warn("Prefer jj over git status.");
        assert_eq!(
            envelope,
            "{\"systemMessage\":\"Prefer jj over git status.\"}"
        );
    }

    #[test]
    fn adapter_failure_is_a_bare_system_message() {
        let envelope = adapter_failure("vcs-detect: probe failed");
        assert_eq!(
            envelope,
            "{\"systemMessage\":\"vcs-detect: probe failed\"}"
        );
    }
}
