//! The byte-exact `## Project Context` / `## Skill-Specific Context` blocks.
//!
//! Each block is rendered without a trailing newline; the handler joins the
//! survivors and adds the single final newline. The `--fail-safe` notices are
//! the bare `Unavailable` headers.

const PROJECT_PROSE: &str = "\
The following project-specific context has been provided. Take this into
account when making decisions, selecting approaches, and generating output.";

pub const PROJECT_UNAVAILABLE: &str = "## Project Context Unavailable";
pub const SKILL_UNAVAILABLE: &str = "## Skill-Specific Context Unavailable";

#[must_use]
pub fn project(body: &str) -> String {
    format!("## Project Context\n\n{PROJECT_PROSE}\n\n{body}")
}

#[must_use]
pub fn skill(name: &str, content: &str) -> String {
    let prose = format!(
        "The following context is specific to the {name} skill. Apply this"
    );
    format!(
        "## Skill-Specific Context\n\n{prose}\ncontext in addition to any \
         project-wide context above.\n\n{content}"
    )
}

#[cfg(test)]
mod tests {
    use super::{project, skill};

    #[test]
    fn the_project_block_ends_at_the_body_with_no_trailing_newline() {
        assert_eq!(
            project("only"),
            "## Project Context\n\nThe following project-specific context has \
             been provided. Take this into\naccount when making decisions, \
             selecting approaches, and generating output.\n\nonly"
        );
    }

    #[test]
    fn the_skill_block_interpolates_the_name() {
        let block = skill("create-plan", "body");
        assert!(block.starts_with("## Skill-Specific Context\n\n"));
        assert!(block.contains("specific to the create-plan skill"));
        assert!(block.ends_with("above.\n\nbody"));
    }
}
