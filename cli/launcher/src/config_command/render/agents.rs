//! The byte-exact `## Agent Names` block and its `--fail-safe` notice.

use crate::config_command::core::agents::AgentsView;
use crate::config_command::render::Rendered;

const PROSE: &str = "\
The following agent names are configured for this project. Always use
the name shown for each role as the `subagent_type` parameter when
spawning agents via the Agent/Task tool.";

#[must_use]
pub fn render(view: &AgentsView) -> Rendered {
    let mut stdout = String::from("## Agent Names\n\n");
    stdout.push_str(PROSE);
    stdout.push_str("\n\n");
    for agent in &view.agents {
        stdout.push_str("- **");
        stdout.push_str(&agent.name.replace('-', " "));
        stdout.push_str(" agent**: ");
        stdout.push_str(&agent.value);
        stdout.push('\n');
    }
    let warnings = view
        .unknown
        .iter()
        .map(|key| format!("Warning: unknown agent key '{key}' — ignoring"))
        .collect();
    Rendered { stdout, warnings }
}

#[must_use]
pub fn render_unavailable() -> Rendered {
    super::unavailable("## Agent Names Unavailable")
}

#[cfg(test)]
mod tests {
    use super::{render, render_unavailable};
    use crate::config_command::core::agents::{Agent, AgentsView};

    fn agent(name: &str, value: &str) -> Agent {
        Agent {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    #[test]
    fn renders_the_header_prose_and_bullets_ending_in_a_newline() {
        let view = AgentsView {
            agents: vec![
                agent("reviewer", "accelerator:reviewer"),
                agent("codebase-locator", "my-locator"),
            ],
            unknown: Vec::new(),
        };
        let rendered = render(&view);
        assert!(rendered.stdout.starts_with("## Agent Names\n\n"));
        assert!(rendered
            .stdout
            .contains("spawning agents via the Agent/Task tool.\n\n"));
        assert!(rendered
            .stdout
            .ends_with("- **codebase locator agent**: my-locator\n"));
        assert!(rendered.warnings.is_empty());
    }

    #[test]
    fn an_unknown_key_becomes_a_stderr_warning() {
        let view = AgentsView {
            agents: Vec::new(),
            unknown: vec!["bogus".to_owned()],
        };
        let rendered = render(&view);
        assert_eq!(rendered.warnings.len(), 1);
        assert!(rendered.warnings[0].contains("unknown agent key 'bogus'"));
    }

    #[test]
    fn the_unavailable_notice_is_the_bare_header() {
        assert_eq!(render_unavailable().stdout, "## Agent Names Unavailable\n");
    }
}
