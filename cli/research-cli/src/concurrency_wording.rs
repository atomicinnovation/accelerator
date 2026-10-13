//! How a resolved concurrency's and subagent cap's diagnostics read to
//! whoever runs `conduct`.

use config::Level;
use research::conduct::concurrency::ConcurrencyWarning;
use research::conduct::concurrency::ConfigFile;
use research::conduct::concurrency::EmptyConcurrency;
use research::conduct::concurrency::Provenance;

use crate::subagent_cap::CapSource;
use crate::subagent_cap::UnhonouredCap;

pub fn unhonoured_cap(unhonoured: &UnhonouredCap) -> String {
    format!(
        "{} is '{}', not a positive integer — ignoring it",
        unhonoured.variable, unhonoured.value
    )
}

pub fn concurrency_warning(
    warning: &ConcurrencyWarning,
    cap_source: CapSource,
) -> String {
    match warning {
        ConcurrencyWarning::NotAPositiveInteger { given, provenance } => {
            format!(
                "{} must be a positive integer, got '{given}' — clamping to 1",
                concurrency_source(*provenance)
            )
        }
        ConcurrencyWarning::AboveCap {
            given,
            provenance,
            cap,
        } => {
            let cap = cap.slots();
            let source = concurrency_source(*provenance);
            let raised_by = cap_source.raised_by();
            match cap_source {
                CapSource::Variable(_) => format!(
                    "{source} is {given}, above the subagent cap of {cap} \
                     from {raised_by} — clamping to {cap}; lower it, or raise \
                     {raised_by}"
                ),
                CapSource::HarnessDefault => format!(
                    "{source} is {given}, above the subagent cap of {cap}, \
                     Claude Code's default — clamping to {cap}; lower it, or \
                     raise the cap with {raised_by}"
                ),
            }
        }
    }
}

pub fn empty_concurrency(empty: EmptyConcurrency) -> String {
    let source = concurrency_source(empty.provenance);
    match empty.provenance {
        Provenance::Flag => {
            format!("{source} is empty; pass a positive integer")
        }
        Provenance::ConfigFile(file) => format!(
            "{source} in {} is empty; set it to a positive integer or remove \
             it",
            path_of(file)
        ),
    }
}

const fn path_of(file: ConfigFile) -> &'static str {
    match file {
        ConfigFile::Team => Level::Team.filename(),
        ConfigFile::Personal => Level::Personal.filename(),
    }
}

const fn concurrency_source(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::Flag => "--concurrency",
        Provenance::ConfigFile(_) => "research.topic.concurrency",
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::num::NonZeroU32;

    use research::conduct::concurrency::ConcurrencyWarning;
    use research::conduct::concurrency::ConfigFile;
    use research::conduct::concurrency::EmptyConcurrency;
    use research::conduct::concurrency::Provenance;
    use research::conduct::concurrency::SubagentCap;

    use super::concurrency_warning;
    use super::empty_concurrency;
    use super::CapSource;

    #[test]
    fn every_concurrency_diagnostic_names_its_source() {
        let cap =
            SubagentCap::new(NonZeroU32::new(20).expect("a non-zero cap"));
        let accelerator_cap = CapSource::Variable("ACCELERATOR_SUBAGENT_CAP");
        let rows = [
            (
                Provenance::Flag,
                "--concurrency must be a positive integer, got 'many' — \
                 clamping to 1",
                "--concurrency is 50, above the subagent cap of 20 from \
                 ACCELERATOR_SUBAGENT_CAP — clamping to 20; lower it, or \
                 raise ACCELERATOR_SUBAGENT_CAP",
                "--concurrency is empty; pass a positive integer",
            ),
            (
                Provenance::ConfigFile(ConfigFile::Team),
                "research.topic.concurrency must be a positive integer, got \
                 'many' — clamping to 1",
                "research.topic.concurrency is 50, above the subagent cap of \
                 20 from ACCELERATOR_SUBAGENT_CAP — clamping to 20; lower it, \
                 or raise ACCELERATOR_SUBAGENT_CAP",
                "research.topic.concurrency in .accelerator/config.md is \
                 empty; set it to a positive integer or remove it",
            ),
        ];
        for (provenance, invalid, above_cap, empty) in rows {
            assert_eq!(
                concurrency_warning(
                    &ConcurrencyWarning::NotAPositiveInteger {
                        given: "many".into(),
                        provenance,
                    },
                    accelerator_cap,
                ),
                invalid,
                "{provenance:?}"
            );
            assert_eq!(
                concurrency_warning(
                    &ConcurrencyWarning::AboveCap {
                        given: "50".into(),
                        provenance,
                        cap,
                    },
                    accelerator_cap,
                ),
                above_cap,
                "{provenance:?}"
            );
            assert_eq!(
                empty_concurrency(EmptyConcurrency { provenance }),
                empty,
                "{provenance:?}"
            );
        }
    }

    #[test]
    fn a_clamp_to_the_harness_default_says_how_to_raise_it() {
        let cap =
            SubagentCap::new(NonZeroU32::new(20).expect("a non-zero cap"));
        let raised_by = CapSource::HarnessDefault.raised_by();

        assert_eq!(
            concurrency_warning(
                &ConcurrencyWarning::AboveCap {
                    given: "24".into(),
                    provenance: Provenance::ConfigFile(ConfigFile::Team),
                    cap,
                },
                CapSource::HarnessDefault,
            ),
            format!(
                "research.topic.concurrency is 24, above the subagent cap of \
                 20, Claude Code's default — clamping to 20; lower it, or \
                 raise the cap with {raised_by}"
            )
        );
    }
}
