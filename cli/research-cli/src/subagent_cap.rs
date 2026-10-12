//! The subagent cap, read from the environment `conduct` runs in.

use std::num::NonZeroU32;

use research::conduct::concurrency::SubagentCap;

const ACCELERATOR_CAP: &str = "ACCELERATOR_SUBAGENT_CAP";
const HARNESS_CAPS: [&str; 1] = ["CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"];

pub const HARNESS_DEFAULT: SubagentCap =
    SubagentCap::new(match NonZeroU32::new(20) {
        Some(slots) => slots,
        None => NonZeroU32::MIN,
    });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapSource {
    Variable(&'static str),
    HarnessDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentCap {
    pub cap: SubagentCap,
    pub source: CapSource,
}

/// Each variable is read as the harness's JavaScript parser reads its own:
/// trimmed as `String.prototype.trim` trims, an optionally signed run of
/// ASCII digits, at least 1.
pub fn from_environment(
    lookup: impl Fn(&str) -> Option<String>,
) -> EnvironmentCap {
    std::iter::once(ACCELERATOR_CAP)
        .chain(HARNESS_CAPS)
        .find_map(|name| {
            let slots = harness_slots(&lookup(name)?)?;
            Some(EnvironmentCap {
                cap: SubagentCap::new(slots),
                source: CapSource::Variable(name),
            })
        })
        .unwrap_or(EnvironmentCap {
            cap: HARNESS_DEFAULT,
            source: CapSource::HarnessDefault,
        })
}

fn harness_slots(setting: &str) -> Option<NonZeroU32> {
    let trimmed = setting.trim_matches(is_javascript_whitespace);
    let digits = trimmed.strip_prefix('+').unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    NonZeroU32::new(digits.parse().unwrap_or(u32::MAX))
}

/// JavaScript's `trim` keeps U+0085, which Rust counts as whitespace, and
/// strips U+FEFF, which Rust does not.
const fn is_javascript_whitespace(character: char) -> bool {
    (character.is_whitespace() && character != '\u{85}')
        || character == '\u{FEFF}'
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use config::catalogue::default_for;
    use config::render_value;
    use research::conduct::concurrency::BUILT_IN_CONCURRENCY;

    use super::from_environment;
    use super::CapSource;
    use super::EnvironmentCap;
    use super::ACCELERATOR_CAP;
    use super::HARNESS_DEFAULT;

    const HARNESS_CAP: &str = "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS";

    fn environment_cap(variables: &[(&str, &str)]) -> EnvironmentCap {
        let environment: HashMap<_, _> = variables.iter().copied().collect();
        from_environment(|name| {
            environment.get(name).map(|value| (*value).to_owned())
        })
    }

    fn cap_in(variables: &[(&str, &str)]) -> u32 {
        environment_cap(variables).cap.slots().get()
    }

    #[test]
    fn the_cap_names_the_variable_it_was_read_from() {
        let rows: [(&[(&str, &str)], CapSource); 4] = [
            (
                &[(ACCELERATOR_CAP, "30"), (HARNESS_CAP, "8")],
                CapSource::Variable(ACCELERATOR_CAP),
            ),
            (
                &[(ACCELERATOR_CAP, "abc"), (HARNESS_CAP, "8")],
                CapSource::Variable(HARNESS_CAP),
            ),
            (&[(ACCELERATOR_CAP, "0")], CapSource::HarnessDefault),
            (&[], CapSource::HarnessDefault),
        ];
        for (variables, source) in rows {
            assert_eq!(
                environment_cap(variables).source,
                source,
                "{variables:?}"
            );
        }
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
        assert_eq!(cap_in(&[]), HARNESS_DEFAULT.slots().get());
    }

    #[test]
    fn a_cap_is_parsed_as_the_harness_parses_it() {
        let zero_padded = format!("{}8", "0".repeat(40));
        for (setting, slots) in [
            ("8", 8),
            ("007", 7),
            ("+8", 8),
            (" 8 ", 8),
            ("\t8\n", 8),
            ("\u{FEFF}8", 8),
            ("30", 30),
            ("1", 1),
            (zero_padded.as_str(), 8),
        ] {
            assert_eq!(
                cap_in(&[(ACCELERATOR_CAP, setting)]),
                slots,
                "cap {setting:?}"
            );
        }
    }

    #[test]
    fn a_cap_the_harness_ignores_is_the_default() {
        for setting in [
            "0", "-5", "-0", "-", "abc", "20.5", "1e2", "1,000", "٨",
            "\u{85}8", "", "  ", "+",
        ] {
            assert_eq!(
                cap_in(&[(ACCELERATOR_CAP, setting)]),
                HARNESS_DEFAULT.slots().get(),
                "cap {setting:?}"
            );
        }
    }

    #[test]
    fn a_cap_too_large_for_u32_saturates() {
        assert_eq!(cap_in(&[(ACCELERATOR_CAP, "99999999999")]), u32::MAX);
    }

    #[test]
    fn the_built_in_concurrency_sits_at_the_harness_default_cap() {
        assert_eq!(BUILT_IN_CONCURRENCY, HARNESS_DEFAULT.slots());
        assert_eq!(
            default_for("research.topic.concurrency")
                .as_ref()
                .map(render_value),
            Some(BUILT_IN_CONCURRENCY.to_string())
        );
    }
}
