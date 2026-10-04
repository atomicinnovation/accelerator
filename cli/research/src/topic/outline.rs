//! The focus areas an `outline.md` asks for, and the profiles each is
//! researched through.

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
}
