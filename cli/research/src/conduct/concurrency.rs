//! How many agents `conduct` spawns at once: the requested concurrency,
//! validated, then bounded by the harness's subagent cap.

use std::num::NonZeroU32;

pub const BUILT_IN_CONCURRENCY: NonZeroU32 = match NonZeroU32::new(20) {
    Some(agents) => agents,
    None => NonZeroU32::MIN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubagentCap(NonZeroU32);

impl SubagentCap {
    #[must_use]
    pub const fn new(slots: NonZeroU32) -> Self {
        Self(slots)
    }

    #[must_use]
    pub const fn slots(self) -> NonZeroU32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Flag,
    ConfigFile(ConfigFile),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFile {
    Team,
    Personal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestedConcurrency {
    Given {
        value: String,
        provenance: Provenance,
    },
    BuiltInDefault,
}

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
        match self {
            Self::Given { value, provenance } => {
                resolve_given(value, *provenance, cap)
            }
            Self::BuiltInDefault => Ok(ResolvedConcurrency {
                concurrency: Concurrency::new(
                    BUILT_IN_CONCURRENCY.min(cap.slots()),
                ),
                warnings: Vec::new(),
            }),
        }
    }
}

fn resolve_given(
    value: &str,
    provenance: Provenance,
    cap: SubagentCap,
) -> Result<ResolvedConcurrency, EmptyConcurrency> {
    if value.is_empty() {
        return Err(EmptyConcurrency { provenance });
    }
    let mut warnings = Vec::new();
    let valid = positive_integer(value);
    if valid.is_none() {
        warnings.push(ConcurrencyWarning::NotAPositiveInteger {
            given: value.to_owned(),
            provenance,
        });
    }
    let requested = valid.unwrap_or(NonZeroU32::MIN);
    let agents = requested.min(cap.slots());
    if agents < requested {
        warnings.push(ConcurrencyWarning::AboveCap {
            given: value.to_owned(),
            provenance,
            cap,
        });
    }
    Ok(ResolvedConcurrency {
        concurrency: Concurrency::new(agents),
        warnings,
    })
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
