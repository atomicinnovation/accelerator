//! How the search cap's diagnostics read to whoever runs `conduct`.

use research::conduct::search_budget::SearchBudgets;
use research::conduct::search_budget::SearchCap;

use crate::search_cap::EnvironmentSearchCap;
use crate::search_cap::SearchCapSource;

pub fn search_floor(
    budgets: &SearchBudgets,
    search_cap: &EnvironmentSearchCap,
) -> String {
    let cap = search_cap.cap;
    let source = match search_cap.source {
        SearchCapSource::Variable => SearchCapSource::raised_by(),
        SearchCapSource::HarnessDefault => "the default",
    };
    format!(
        "the worst-case demand of {} web searches exceeds the {} left by the \
         search cap of {} from {source} after a reserve of {} — every search \
         budget is 1; for later runs, raise {} and restart Claude Code, or \
         lower the depth",
        budgets.demand(),
        cap.available(),
        cap.searches(),
        cap.reserve(),
        SearchCapSource::raised_by(),
    )
}

pub fn unhonoured_search_cap(value: &str, cap: SearchCap) -> String {
    format!(
        "{} is '{value}', not a positive integer — ignoring it; sizing search \
         budgets against the search cap of {} from the default",
        SearchCapSource::raised_by(),
        cap.searches(),
    )
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use research::conduct::search_budget::SearchBudgets;
    use research::conduct::search_budget::SearchCap;

    use super::search_floor;
    use super::unhonoured_search_cap;
    use crate::search_cap::EnvironmentSearchCap;
    use crate::search_cap::SearchCapSource;
    use crate::search_cap::HARNESS_DEFAULT;

    #[test]
    fn an_unhonoured_search_cap_names_the_value_and_the_default() {
        assert_eq!(
            unhonoured_search_cap("abc", HARNESS_DEFAULT),
            format!(
                "{} is 'abc', not a positive integer — ignoring it; sizing \
                 search budgets against the search cap of 200 from the \
                 default",
                SearchCapSource::raised_by()
            )
        );
    }

    fn ones_demanding(demand: u64) -> SearchBudgets {
        let levels = (1..=3).map(|level| (level, NonZeroU32::MIN)).collect();
        SearchBudgets::from_parts(levels, demand)
    }

    fn search_cap(
        searches: u32,
        source: SearchCapSource,
    ) -> EnvironmentSearchCap {
        EnvironmentSearchCap {
            cap: SearchCap::new(
                NonZeroU32::new(searches).unwrap_or(NonZeroU32::MIN),
            ),
            source,
            unhonoured: None,
        }
    }

    #[test]
    fn the_floor_warning_names_the_demand_the_cap_and_where_it_came_from() {
        let variable = SearchCapSource::raised_by();
        let rows = [
            (
                ones_demanding(260),
                search_cap(200, SearchCapSource::HarnessDefault),
                "the worst-case demand of 260 web searches exceeds the 190 \
                 left by the search cap of 200 from the default after a \
                 reserve of 10"
                    .to_owned(),
            ),
            (
                ones_demanding(70),
                search_cap(60, SearchCapSource::Variable),
                format!(
                    "the worst-case demand of 70 web searches exceeds the 57 \
                     left by the search cap of 60 from {variable} after a \
                     reserve of 3"
                ),
            ),
        ];
        for (budgets, cap, shortfall) in rows {
            assert_eq!(
                search_floor(&budgets, &cap),
                format!(
                    "{shortfall} — every search budget is 1; for later runs, \
                     raise {variable} and restart Claude Code, or lower the \
                     depth"
                )
            );
        }
    }
}
