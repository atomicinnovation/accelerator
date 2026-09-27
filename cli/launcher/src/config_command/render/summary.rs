//! The `summary` output: the human-facing body assembled from the view, then
//! optionally wrapped in the `SessionStart` hook envelope.

use config::Level;

use crate::config_command::core::summary::{Summary, SummaryView};

const INIT_HINT: &str = "Accelerator has not been initialised in this \
repository. Type /accelerator:init at the prompt to set up the expected \
directory structure and gitignore entries.";

const TRAILER: &str = "Skills will read this configuration at invocation \
time. To view or edit configuration, use /accelerator:configure.";

/// The human-facing summary body, or `None` when there is nothing to inject.
#[must_use]
pub fn body(summary: &Summary) -> Option<String> {
    match summary {
        Summary::Nothing => None,
        Summary::NotInitialised => Some(INIT_HINT.to_owned()),
        Summary::Configured(view) => Some(configured_body(view)),
    }
}

fn configured_body(view: &SummaryView) -> String {
    let mut summary =
        String::from("Accelerator plugin configuration detected:");
    for level in &view.present_levels {
        summary.push_str(match level {
            Level::Team => "\n- Team config: ",
            Level::Personal => "\n- Personal config: ",
        });
        summary.push_str(level.filename());
    }
    if !view.configured_sections.is_empty() {
        summary.push_str("\n- Configured sections:");
        for section in &view.configured_sections {
            summary.push(' ');
            summary.push_str(section);
        }
    }
    if view.has_project_context {
        summary.push_str(
            "\n- Project context: provided (will be injected into skills)",
        );
    }
    if !view.customisations.is_empty() {
        summary.push_str("\n- Per-skill customisations:");
        for line in &view.customisations {
            summary.push_str("\n    - ");
            summary.push_str(line);
        }
    }
    summary.push_str("\n\n");
    summary.push_str(TRAILER);
    if !view.initialised {
        summary.push_str("\n\n");
        summary.push_str(INIT_HINT);
    }
    summary
}

/// Wraps a summary in the compact `SessionStart` envelope.
///
/// Session warnings follow the summary in `additionalContext` and also reach
/// the user as the `systemMessage`; without any, the envelope carries the
/// summary alone.
#[must_use]
pub fn hook_envelope(summary: Option<&str>, session: &[String]) -> String {
    let warnings = session.join("\n");
    let context = match summary {
        Some(summary) if !warnings.is_empty() => {
            format!("{summary}\n\n{warnings}")
        }
        Some(summary) => summary.to_owned(),
        None => warnings,
    };
    let system_message = session
        .iter()
        .map(|warning| format!("[accelerator] {warning}"))
        .collect::<Vec<_>>()
        .join("\n");
    kernel::hooks::session_start(&context, Some(&system_message))
}

#[cfg(test)]
mod tests {
    use super::hook_envelope;

    #[test]
    fn session_warnings_reach_both_fields() {
        let envelope =
            hook_envelope(Some("summary"), &["E_X: warned".to_owned()]);
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"summary\\n\\nE_X: warned\"},\
             \"systemMessage\":\"[accelerator] E_X: warned\"}"
        );
    }

    #[test]
    fn session_warnings_alone_still_make_an_envelope() {
        let envelope = hook_envelope(None, &["E_X: warned".to_owned()]);
        assert!(envelope.contains("\"additionalContext\":\"E_X: warned\""));
        assert!(envelope.contains("\"systemMessage\""));
    }

    #[test]
    fn the_envelope_carries_the_summary_as_additional_context() {
        let envelope = hook_envelope(Some("line one\nline two"), &[]);
        assert_eq!(
            envelope,
            "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\
             \"additionalContext\":\"line one\\nline two\"}}"
        );
    }
}
