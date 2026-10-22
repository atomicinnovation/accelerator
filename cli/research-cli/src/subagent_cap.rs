//! The subagent cap, read from the environment `conduct` runs in.

use std::num::NonZeroU32;

use research::conduct::concurrency::SubagentCap;

use crate::harness_setting::is_set;
use crate::harness_setting::positive_whole_number;

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

impl CapSource {
    pub const fn raised_by(self) -> &'static str {
        match self {
            Self::Variable(name) => name,
            Self::HarnessDefault => HARNESS_CAPS[0],
        }
    }

    pub const fn reported_as(self) -> &'static str {
        match self {
            Self::Variable(name) => name,
            Self::HarnessDefault => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentCap {
    pub cap: SubagentCap,
    pub source: CapSource,
    pub unhonoured: Option<UnhonouredCap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnhonouredCap {
    pub variable: &'static str,
    pub value: String,
}

pub fn from_environment(
    lookup: impl Fn(&str) -> Option<String>,
) -> EnvironmentCap {
    let accelerator = lookup(ACCELERATOR_CAP);
    let unhonoured = accelerator
        .as_deref()
        .filter(|value| is_set(value) && positive_whole_number(value).is_none())
        .map(|value| UnhonouredCap {
            variable: ACCELERATOR_CAP,
            value: value.to_owned(),
        });
    let honoured = std::iter::once((ACCELERATOR_CAP, accelerator))
        .chain(HARNESS_CAPS.into_iter().map(|name| (name, lookup(name))))
        .find_map(|(name, setting)| {
            Some((CapSource::Variable(name), positive_whole_number(&setting?)?))
        });
    let (source, cap) = honoured.map_or(
        (CapSource::HarnessDefault, HARNESS_DEFAULT),
        |(source, slots)| (source, SubagentCap::new(slots)),
    );
    EnvironmentCap {
        cap,
        source,
        unhonoured,
    }
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
    use super::UnhonouredCap;
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
    fn an_accelerator_cap_the_harness_would_not_honour_is_reported() {
        for value in ["abc", "0", "-5"] {
            assert_eq!(
                environment_cap(&[
                    (ACCELERATOR_CAP, value),
                    (HARNESS_CAP, "8")
                ])
                .unhonoured,
                Some(UnhonouredCap {
                    variable: ACCELERATOR_CAP,
                    value: value.to_owned(),
                }),
                "{value:?}"
            );
        }
    }

    #[test]
    fn a_blank_or_honoured_accelerator_cap_and_any_harness_cap_go_unreported() {
        let rows: [&[(&str, &str)]; 5] = [
            &[(ACCELERATOR_CAP, "")],
            &[(ACCELERATOR_CAP, "  ")],
            &[(ACCELERATOR_CAP, "8")],
            &[(HARNESS_CAP, "abc")],
            &[],
        ];
        for variables in rows {
            assert_eq!(
                environment_cap(variables).unhonoured,
                None,
                "{variables:?}"
            );
        }
    }

    #[test]
    fn the_harness_default_is_raised_by_the_harness_variable() {
        assert_eq!(CapSource::HarnessDefault.raised_by(), HARNESS_CAP);
        assert_eq!(
            CapSource::Variable(ACCELERATOR_CAP).raised_by(),
            ACCELERATOR_CAP
        );
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
