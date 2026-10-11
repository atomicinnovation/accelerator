//! The subagent cap, read from the environment `conduct` runs in.

use research::conduct::concurrency::SubagentCap;

const ACCELERATOR_CAP: &str = "ACCELERATOR_SUBAGENT_CAP";
const HARNESS_CAPS: [&str; 1] = ["CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"];

/// The first cap variable holding a value the harness would honour, the
/// accelerator's own before any harness's, else the harness default.
pub fn from_environment(
    lookup: impl Fn(&str) -> Option<String>,
) -> SubagentCap {
    let settings: Vec<Option<String>> = std::iter::once(ACCELERATOR_CAP)
        .chain(HARNESS_CAPS)
        .map(&lookup)
        .collect();
    SubagentCap::first_honoured(settings.iter().map(Option::as_deref))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use research::conduct::concurrency::SubagentCap;

    use super::from_environment;

    fn cap_in(variables: &[(&str, &str)]) -> u32 {
        let environment: HashMap<_, _> = variables.iter().copied().collect();
        from_environment(|name| {
            environment.get(name).map(|value| (*value).to_owned())
        })
        .slots()
        .get()
    }

    #[test]
    fn the_accelerator_cap_wins_over_the_harness_cap() {
        assert_eq!(
            cap_in(&[
                ("ACCELERATOR_SUBAGENT_CAP", "30"),
                ("CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS", "8"),
            ]),
            30
        );
    }

    #[test]
    fn an_accelerator_cap_the_harness_would_ignore_falls_through() {
        assert_eq!(
            cap_in(&[
                ("ACCELERATOR_SUBAGENT_CAP", "abc"),
                ("CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS", "8"),
            ]),
            8
        );
    }

    #[test]
    fn the_harness_cap_alone_is_read() {
        assert_eq!(cap_in(&[("CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS", "8")]), 8);
    }

    #[test]
    fn without_either_variable_the_cap_is_the_harness_default() {
        assert_eq!(cap_in(&[]), SubagentCap::HARNESS_DEFAULT.slots().get());
    }
}
