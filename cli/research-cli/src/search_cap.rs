//! The session's `WebSearch` cap, read from the environment `conduct` runs
//! in.

use std::num::NonZeroU32;

use research::conduct::search_budget::SearchCap;

use crate::harness_setting::is_set;
use crate::harness_setting::positive_whole_number;

const HARNESS_SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";

pub const HARNESS_DEFAULT: SearchCap =
    SearchCap::new(match NonZeroU32::new(200) {
        Some(searches) => searches,
        None => NonZeroU32::MIN,
    });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchCapSource {
    Variable,
    HarnessDefault,
}

impl SearchCapSource {
    pub const fn raised_by() -> &'static str {
        HARNESS_SEARCH_CAP
    }

    pub const fn reported_as(self) -> &'static str {
        match self {
            Self::Variable => HARNESS_SEARCH_CAP,
            Self::HarnessDefault => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentSearchCap {
    pub cap: SearchCap,
    pub source: SearchCapSource,
    pub unhonoured: Option<String>,
}

pub fn from_environment(
    lookup: impl Fn(&str) -> Option<String>,
) -> EnvironmentSearchCap {
    let setting = lookup(HARNESS_SEARCH_CAP);
    let honoured = setting.as_deref().and_then(positive_whole_number);
    let unhonoured =
        setting.filter(|value| is_set(value) && honoured.is_none());
    let (cap, source) = honoured.map_or(
        (HARNESS_DEFAULT, SearchCapSource::HarnessDefault),
        |searches| (SearchCap::new(searches), SearchCapSource::Variable),
    );
    EnvironmentSearchCap {
        cap,
        source,
        unhonoured,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::from_environment;
    use super::EnvironmentSearchCap;
    use super::SearchCapSource;
    use super::HARNESS_DEFAULT;

    const HARNESS_SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";

    fn search_cap(setting: Option<&str>) -> EnvironmentSearchCap {
        let environment: HashMap<_, _> = setting
            .map(|value| (HARNESS_SEARCH_CAP, value.to_owned()))
            .into_iter()
            .collect();
        from_environment(|name| environment.get(name).cloned())
    }

    fn searches(cap: &EnvironmentSearchCap) -> u32 {
        cap.cap.searches().get()
    }

    #[test]
    fn an_unset_search_cap_is_the_harness_default_of_200() {
        let cap = search_cap(None);

        assert_eq!(searches(&cap), 200);
        assert_eq!(cap.source, SearchCapSource::HarnessDefault);
        assert_eq!(cap.unhonoured, None);
    }

    #[test]
    fn a_search_cap_is_read_as_the_harness_reads_it() {
        for (setting, expected) in
            [("500", 500), ("+60", 60), (" 192 ", 192), ("0201", 201)]
        {
            let cap = search_cap(Some(setting));

            assert_eq!(searches(&cap), expected, "{setting:?}");
            assert_eq!(cap.source, SearchCapSource::Variable, "{setting:?}");
            assert_eq!(cap.unhonoured, None, "{setting:?}");
        }
    }

    #[test]
    fn a_search_cap_the_harness_ignores_is_the_default_and_reported() {
        for setting in ["abc", "0", "-5", "1.5", "1e2"] {
            let cap = search_cap(Some(setting));

            assert_eq!(searches(&cap), 200, "{setting:?}");
            assert_eq!(
                cap.source,
                SearchCapSource::HarnessDefault,
                "{setting:?}"
            );
            assert_eq!(cap.unhonoured.as_deref(), Some(setting));
        }
    }

    #[test]
    fn a_blank_search_cap_is_unset_and_unreported() {
        for setting in ["", "  "] {
            let cap = search_cap(Some(setting));

            assert_eq!(searches(&cap), 200, "{setting:?}");
            assert_eq!(cap.unhonoured, None, "{setting:?}");
        }
    }

    #[test]
    fn the_default_search_cap_matches_claude_codes() {
        assert_eq!(HARNESS_DEFAULT.searches().get(), 200);
    }

    #[test]
    fn the_source_names_the_variable_that_raises_the_cap() {
        assert_eq!(SearchCapSource::raised_by(), HARNESS_SEARCH_CAP);
        assert_eq!(SearchCapSource::Variable.reported_as(), HARNESS_SEARCH_CAP);
        assert_eq!(SearchCapSource::HarnessDefault.reported_as(), "default");
    }
}
