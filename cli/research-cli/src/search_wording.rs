//! How the search cap's diagnostics read to whoever runs `conduct`.

use research::conduct::search_budget::SearchCap;

use crate::search_cap::SearchCapSource;

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
    use super::unhonoured_search_cap;
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
}
