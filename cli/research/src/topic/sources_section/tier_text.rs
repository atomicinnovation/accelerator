//! Tier text: a base tier with its retraction marks and `also` tiers.

use std::fmt;

use crate::sources::record::Tier;

const RETRACTED: &str = "retracted";
const WITHDRAWN: &str = "withdrawn";
const ALSO: &str = "also ";
const PART_SEPARATOR: &str = "; ";
const ALSO_SEPARATOR: &str = ", ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierText {
    base: Tier,
    retracted: bool,
    withdrawn: bool,
    also: Vec<Tier>,
}

impl TierText {
    /// Checks shape only, so `parse(s)?.to_string() == s`.
    ///
    /// # Errors
    ///
    /// [`NotTierText`], carrying the whole input, for text not of tier-text
    /// shape.
    pub fn parse(text: &str) -> Result<Self, NotTierText> {
        let refused = || NotTierText(text.to_owned());
        let Some((base_text, suffix)) = text.split_once(' ') else {
            return Tier::named(text).map(Self::bare).ok_or_else(refused);
        };
        let base = Tier::named(base_text).ok_or_else(refused)?;
        let parenthesised = suffix
            .strip_prefix('(')
            .and_then(|inner| inner.strip_suffix(')'))
            .ok_or_else(refused)?;
        Self::with_parts_in_canonical_order(base, parenthesised)
            .ok_or_else(refused)
    }

    const fn bare(base: Tier) -> Self {
        Self {
            base,
            retracted: false,
            withdrawn: false,
            also: Vec::new(),
        }
    }

    fn with_parts_in_canonical_order(
        base: Tier,
        parenthesised: &str,
    ) -> Option<Self> {
        let mut parts = parenthesised.split(PART_SEPARATOR).peekable();
        let retracted = parts.next_if_eq(&RETRACTED).is_some();
        let withdrawn = parts.next_if_eq(&WITHDRAWN).is_some();
        let also = match parts.next() {
            Some(part) => also_tiers(part)?,
            None => Vec::new(),
        };
        parts.next().is_none().then_some(Self {
            base,
            retracted,
            withdrawn,
            also,
        })
    }

    #[must_use]
    pub const fn base(&self) -> Tier {
        self.base
    }

    #[must_use]
    pub const fn retracted(&self) -> bool {
        self.retracted
    }

    #[must_use]
    pub const fn withdrawn(&self) -> bool {
        self.withdrawn
    }

    #[must_use]
    pub fn also(&self) -> &[Tier] {
        &self.also
    }
}

fn also_tiers(part: &str) -> Option<Vec<Tier>> {
    part.strip_prefix(ALSO)?
        .split(ALSO_SEPARATOR)
        .map(Tier::named)
        .collect()
}

impl fmt::Display for TierText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let also = (!self.also.is_empty()).then(|| {
            let tiers: Vec<String> =
                self.also.iter().map(Tier::to_string).collect();
            format!("{ALSO}{}", tiers.join(ALSO_SEPARATOR))
        });
        let parts: Vec<String> = [
            self.retracted.then(|| RETRACTED.to_owned()),
            self.withdrawn.then(|| WITHDRAWN.to_owned()),
            also,
        ]
        .into_iter()
        .flatten()
        .collect();
        write!(formatter, "{}", self.base)?;
        if parts.is_empty() {
            return Ok(());
        }
        write!(formatter, " ({})", parts.join(PART_SEPARATOR))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotTierText(String);

impl NotTierText {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_text(self) -> String {
        self.0
    }
}

impl fmt::Display for NotTierText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "not tier text: `{}`", self.0)
    }
}

impl std::error::Error for NotTierText {}
