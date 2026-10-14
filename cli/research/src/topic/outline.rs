//! The focus areas an `outline.md` asks for, and the profiles each is
//! researched through.

use crate::topic::layout::slug::QuestionSlug;
use crate::topic::layout::stem::Stem;

/// The profile an outline item is researched through when it names none.
pub const DEFAULT_PROFILE: &str = "web";

const SUFFIX_KEYWORD: &str = "profiles:";
const SUFFIX_SEPARATORS: [&str; 3] = ["—", "–", "--"];

/// One focus area researched through one source profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub question: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    pub items: Vec<OutlineItem>,
}

impl Outline {
    /// Every `- [ ]` or `- [x]` checkbox line, ticked or not.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let items = text
            .lines()
            .enumerate()
            .filter_map(|(index, text)| OutlineItem::parse(index + 1, text))
            .collect();
        Self { items }
    }
}

/// One focus area: the one-based line it is on, its question, and the
/// profiles it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    pub line: usize,
    pub question: String,
    suffix: ProfilesSuffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfilesSuffix {
    Absent,
    Named(Vec<String>),
    Unseparated,
}

impl OutlineItem {
    fn parse(line: usize, text: &str) -> Option<Self> {
        let rest = text.trim_start().strip_prefix("- [")?;
        let (mark, text) = rest.split_at_checked(1)?;
        let text = text.strip_prefix(']')?;
        if !matches!(mark, " " | "x" | "X") {
            return None;
        }
        let (question, suffix) = split_suffix(text.trim());
        Some(Self {
            line,
            question,
            suffix,
        })
    }

    #[must_use]
    pub fn profiles(&self) -> Vec<&str> {
        match &self.suffix {
            ProfilesSuffix::Named(profiles) => {
                profiles.iter().map(String::as_str).collect()
            }
            ProfilesSuffix::Absent | ProfilesSuffix::Unseparated => {
                vec![DEFAULT_PROFILE]
            }
        }
    }

    /// Whether the item names `profiles:` with no separator before it, so
    /// the suffix is read as part of its question.
    #[must_use]
    pub fn names_profiles_unseparated(&self) -> bool {
        self.suffix == ProfilesSuffix::Unseparated
    }

    #[must_use]
    pub fn pair(&self, profile: &str) -> Pair {
        Pair {
            question: self.question.clone(),
            profile: profile.to_owned(),
        }
    }

    pub(crate) fn stem_at(&self, index: u32, profile: &str) -> Option<Stem> {
        let slug = QuestionSlug::from(self.question.as_str());
        Stem::allocated(index, slug.as_str(), profile)
    }

    pub(crate) fn pair_at(&self, stem: &Stem) -> Option<Pair> {
        self.profiles()
            .into_iter()
            .find(|profile| {
                self.stem_at(stem.index(), profile).as_ref() == Some(stem)
            })
            .map(|profile| self.pair(profile))
    }

    pub(crate) fn pair_ending(&self, stem: &Stem) -> Option<Pair> {
        self.profiles()
            .into_iter()
            .filter(|profile| stem.ends_with_profile(profile))
            .max_by_key(|profile| profile.len())
            .map(|profile| self.pair(profile))
    }
}

fn split_suffix(text: &str) -> (String, ProfilesSuffix) {
    let Some(keyword_at) = text.rfind(SUFFIX_KEYWORD) else {
        return (text.to_owned(), ProfilesSuffix::Absent);
    };
    let before = text[..keyword_at].trim_end();
    let named = text[keyword_at + SUFFIX_KEYWORD.len()..]
        .split(',')
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let question = SUFFIX_SEPARATORS
        .iter()
        .find_map(|separator| before.strip_suffix(separator));
    match question {
        Some(question) if !named.is_empty() => {
            (question.trim_end().to_owned(), ProfilesSuffix::Named(named))
        }
        _ => (text.to_owned(), ProfilesSuffix::Unseparated),
    }
}

#[cfg(test)]
mod tests {
    use super::Outline;
    use super::OutlineItem;
    use super::Pair;
    use crate::topic::layout::stem::Stem;

    fn only_item(line: &str) -> Option<OutlineItem> {
        Outline::parse(line).items.into_iter().next()
    }

    #[test]
    fn an_item_without_a_suffix_is_researched_on_the_web(
    ) -> Result<(), &'static str> {
        let item =
            only_item("- [ ] How do heads specialise?").ok_or("an item")?;
        assert_eq!(item.question, "How do heads specialise?");
        assert_eq!(item.profiles(), vec!["web"]);
        assert!(!item.names_profiles_unseparated());
        Ok(())
    }

    #[test]
    fn each_recognised_separator_introduces_the_profiles_suffix(
    ) -> Result<(), &'static str> {
        for separator in ["—", "–", "--"] {
            let line =
                format!("- [x] Why? {separator} profiles: web, openalex");
            let item = only_item(&line).ok_or("an item")?;
            assert_eq!(item.question, "Why?");
            assert_eq!(item.profiles(), vec!["web", "openalex"]);
        }
        Ok(())
    }

    #[test]
    fn an_unseparated_profiles_suffix_stays_in_the_question(
    ) -> Result<(), &'static str> {
        let item = only_item("- [ ] Why? profiles: arxiv").ok_or("an item")?;
        assert_eq!(item.question, "Why? profiles: arxiv");
        assert_eq!(item.profiles(), vec!["web"]);
        assert!(item.names_profiles_unseparated());
        Ok(())
    }

    #[test]
    fn a_line_that_is_not_a_checkbox_is_not_an_item() {
        for line in ["## Round 1", "- plain bullet", "- [y] Why?", ""] {
            assert!(only_item(line).is_none(), "{line:?}");
        }
    }

    #[test]
    fn items_carry_their_one_based_line_number() {
        let outline = Outline::parse("---\nkind: outline\n---\n- [ ] A?");
        assert_eq!(outline.items[0].line, 4);
    }

    fn stem(text: &str) -> Option<Stem> {
        Stem::parse(text)
    }

    fn pair(question: &str, profile: &str) -> Pair {
        Pair {
            question: question.to_owned(),
            profile: profile.to_owned(),
        }
    }

    #[test]
    fn an_item_allocates_its_stem_from_its_slug_and_profile(
    ) -> Result<(), &'static str> {
        let item =
            only_item("- [ ] How do heads specialise? — profiles: openalex")
                .ok_or("an item")?;
        assert_eq!(
            item.stem_at(3, "openalex"),
            stem("03-how-do-heads-specialise-openalex")
        );
        assert_eq!(item.stem_at(3, "Web"), None);
        Ok(())
    }

    #[test]
    fn an_item_names_the_pair_whose_stem_it_rebuilds(
    ) -> Result<(), &'static str> {
        let item =
            only_item("- [ ] A? — profiles: web, openalex").ok_or("an item")?;
        let at = stem("07-a-openalex").ok_or("a stem")?;
        assert_eq!(item.pair_at(&at), Some(pair("A?", "openalex")));
        Ok(())
    }

    #[test]
    fn an_item_names_no_pair_for_a_stem_under_a_profile_it_lacks(
    ) -> Result<(), &'static str> {
        let item =
            only_item("- [ ] A? — profiles: web, openalex").ok_or("an item")?;
        let at = stem("07-a-arxiv").ok_or("a stem")?;
        assert_eq!(item.pair_at(&at), None);
        Ok(())
    }

    #[test]
    fn an_item_asks_a_pair_only_under_a_profile_it_names(
    ) -> Result<(), &'static str> {
        let at = stem("02-a-b-web").ok_or("a stem")?;
        let asking = only_item("- [ ] A b?").ok_or("an item")?;
        assert_eq!(asking.pair_at(&at), Some(pair("A b?", "web")));
        let not_asking =
            only_item("- [ ] A? — profiles: web").ok_or("an item")?;
        assert_eq!(not_asking.pair_at(&at), None);
        Ok(())
    }

    #[test]
    fn an_item_names_the_pair_whose_profile_ends_a_stem_its_slug_no_longer_rebuilds(
    ) -> Result<(), &'static str> {
        let item = only_item("- [ ] \u{FF21}? — profiles: web, openalex")
            .ok_or("an item")?;
        let at = stem("01-a-openalex").ok_or("a stem")?;
        assert_eq!(item.pair_at(&at), None);
        assert_eq!(item.pair_ending(&at), Some(pair("\u{FF21}?", "openalex")));
        let elsewhere = stem("01-a-arxiv").ok_or("a stem")?;
        assert_eq!(item.pair_ending(&elsewhere), None);
        Ok(())
    }

    #[test]
    fn an_item_names_the_longest_profile_ending_a_stem_after_its_slug(
    ) -> Result<(), &'static str> {
        let item = only_item("- [ ] \u{FF21}? — profiles: web, deep-web")
            .ok_or("an item")?;
        let ending =
            |text: &str| stem(text).and_then(|at| item.pair_ending(&at));
        assert_eq!(
            ending("01-a-deep-web"),
            Some(pair("\u{FF21}?", "deep-web"))
        );
        assert_eq!(ending("01-a-web"), Some(pair("\u{FF21}?", "web")));
        assert_eq!(ending("01-deep-web"), Some(pair("\u{FF21}?", "web")));
        Ok(())
    }
}
