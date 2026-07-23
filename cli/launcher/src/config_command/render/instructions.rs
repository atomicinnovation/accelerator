//! The byte-exact `## Additional Instructions` block and its `--fail-safe`
//! notice. Rendered without a trailing newline; the handler adds the final one.

pub const UNAVAILABLE: &str = "## Skill Instructions Unavailable";

#[must_use]
pub fn render(name: &str, content: &str) -> String {
    let prose = format!(
        "The following additional instructions have been provided for the\n\
         {name} skill. Follow these instructions in addition to all\n\
         instructions above."
    );
    format!("## Additional Instructions\n\n{prose}\n\n{content}")
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn interpolates_the_name_and_ends_at_the_content() {
        let block = render("create-plan", "do the thing");
        assert!(block.starts_with("## Additional Instructions\n\n"));
        assert!(block.contains("provided for the\ncreate-plan skill."));
        assert!(block.ends_with("instructions above.\n\ndo the thing"));
    }
}
