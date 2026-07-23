//! The oracle projection of a resolved outcome. The value projection itself
//! lives in the `config` domain (`config::render_value`), re-exported here so
//! the shipped output and the parity oracle share one source and cannot drift.
//! Distinct from `document::render`, which round-trips a document body.

pub use config::render_value;
use config::Resolved;

/// The string a resolution miss projects to. The parity oracle passes it to the
/// bash reader as the default argument so a genuine miss compares equal on both
/// sides.
pub const ABSENT_SENTINEL: &str = "__accelerator_config_absent__";

/// Projects a [`Resolved`] outcome, mapping a miss to [`ABSENT_SENTINEL`].
#[must_use]
pub fn render_resolved(resolved: &Resolved) -> String {
    match resolved {
        Resolved::Found(value) => render_value(value),
        Resolved::Absent => ABSENT_SENTINEL.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use config::Resolved;

    use super::{render_resolved, ABSENT_SENTINEL};

    #[test]
    fn renders_a_miss_as_the_sentinel() {
        assert_eq!(render_resolved(&Resolved::Absent), ABSENT_SENTINEL);
    }
}
