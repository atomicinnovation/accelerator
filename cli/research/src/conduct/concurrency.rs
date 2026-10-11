//! How many agents `conduct` spawns at once: the requested concurrency,
//! validated, then bounded by the harness's subagent cap.

use std::num::NonZeroU32;

/// How many subagents the harness runs at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubagentCap(NonZeroU32);

impl SubagentCap {
    pub const HARNESS_DEFAULT: Self = match NonZeroU32::new(20) {
        Some(slots) => Self(slots),
        None => Self(NonZeroU32::MIN),
    };

    /// Reads the harness setting as the harness's JavaScript parser does:
    /// trimmed as `String.prototype.trim` trims, an optionally signed run
    /// of ASCII digits, at least 1; anything else leaves the default.
    #[must_use]
    pub fn from_setting(setting: Option<&str>) -> Self {
        setting
            .and_then(harness_slots)
            .map_or(Self::HARNESS_DEFAULT, Self)
    }

    #[must_use]
    pub const fn slots(self) -> NonZeroU32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Flag,
    ConfigFile,
    BuiltInDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedConcurrency {
    pub value: String,
    pub provenance: Provenance,
}

/// The most agents `conduct` spawns at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concurrency(NonZeroU32);

impl Concurrency {
    #[must_use]
    pub(crate) const fn new(agents: NonZeroU32) -> Self {
        Self(agents)
    }

    #[must_use]
    pub const fn agents(self) -> NonZeroU32 {
        self.0
    }

    #[must_use]
    pub fn admits(self, waiting: usize) -> usize {
        usize::try_from(self.0.get())
            .map_or(waiting, |agents| waiting.min(agents))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyWarning {
    NotAPositiveInteger {
        given: String,
        provenance: Provenance,
    },
    AboveCap {
        given: String,
        provenance: Provenance,
        cap: SubagentCap,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConcurrency {
    pub concurrency: Concurrency,
    pub warnings: Vec<ConcurrencyWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyConcurrency {
    pub provenance: Provenance,
}

impl RequestedConcurrency {
    /// # Errors
    ///
    /// `EmptyConcurrency` when no value was given, which no clamp can
    /// stand in for.
    pub fn resolve(
        &self,
        cap: SubagentCap,
    ) -> Result<ResolvedConcurrency, EmptyConcurrency> {
        if self.value.is_empty() {
            return Err(EmptyConcurrency {
                provenance: self.provenance,
            });
        }
        let mut warnings = Vec::new();
        let valid = positive_integer(&self.value);
        if valid.is_none() {
            warnings.push(ConcurrencyWarning::NotAPositiveInteger {
                given: self.value.clone(),
                provenance: self.provenance,
            });
        }
        let requested = valid.unwrap_or(NonZeroU32::MIN);
        let agents = requested.min(cap.slots());
        if agents < requested && self.provenance != Provenance::BuiltInDefault {
            warnings.push(ConcurrencyWarning::AboveCap {
                given: self.value.clone(),
                provenance: self.provenance,
                cap,
            });
        }
        Ok(ResolvedConcurrency {
            concurrency: Concurrency::new(agents),
            warnings,
        })
    }
}

fn saturating_digits(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(text.bytes().fold(0, |value: u32, digit| {
        value
            .saturating_mul(10)
            .saturating_add(u32::from(digit - b'0'))
    }))
}

fn positive_integer(text: &str) -> Option<NonZeroU32> {
    saturating_digits(text).and_then(NonZeroU32::new)
}

fn harness_slots(setting: &str) -> Option<NonZeroU32> {
    let trimmed = setting.trim_matches(is_javascript_whitespace);
    if trimmed.starts_with('-') {
        return None;
    }
    positive_integer(trimmed.strip_prefix('+').unwrap_or(trimmed))
}

/// JavaScript's `trim` keeps U+0085, which Rust counts as whitespace, and
/// strips U+FEFF, which Rust does not.
const fn is_javascript_whitespace(character: char) -> bool {
    (character.is_whitespace() && character != '\u{85}')
        || character == '\u{FEFF}'
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::num::NonZeroU32;

    use super::Concurrency;

    pub fn concurrency(agents: u32) -> Concurrency {
        Concurrency::new(NonZeroU32::new(agents).unwrap_or(NonZeroU32::MIN))
    }

    pub const fn unbounded() -> Concurrency {
        Concurrency::new(NonZeroU32::MAX)
    }
}
