//! Whether a recorded daemon may be reused, or the state is stale and a fresh
//! one must be spawned.
//!
//! A pure function of (recorded state, observed state), total by construction
//! rather than by enumeration, so no combination is left without an answer.

use crate::executor::daemon_identity::ObservedDaemon;
use crate::executor::daemon_identity::ObservedStartTime;
use crate::executor::daemon_identity::RecordedDaemon;
use crate::executor::daemon_identity::RecordedStartTime;
use crate::executor::daemon_identity::RecordedState;

/// The drift a recorded probe value may show against a fresh one.
///
/// A daemon records its start time a few milliseconds after the kernel forks
/// it, which can cross a whole-second boundary. A one-second drift cannot be a
/// PID recycle, so the tolerance costs nothing and stops the launcher
/// respawning between every command.
const TOLERANCE_SECONDS: u64 = 1;

/// What to do with the state directory's claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reuse {
    /// The recorded daemon is the live one. Run the command against it.
    Reuse(i32),
    /// The state is stale. Remove it and spawn afresh.
    ///
    /// Recovery **never signals**. A contradicted start time proves only that
    /// the live process is not the recorded daemon; it says nothing about what
    /// that process actually is, and on a developer machine a recycled pid is
    /// as likely to be an editor or a build.
    Recover,
}

/// Judges the recorded state against what the host observes.
#[must_use]
pub const fn evaluate(
    recorded: RecordedState,
    observed: ObservedDaemon,
) -> Reuse {
    let RecordedState::Daemon(daemon) = recorded else {
        return Reuse::Recover;
    };
    let ObservedDaemon::Live(observed_start) = observed else {
        return Reuse::Recover;
    };
    if identity_holds(daemon, observed_start) {
        Reuse::Reuse(daemon.pid)
    } else {
        Reuse::Recover
    }
}

/// Provenance decides first, and only a probe value is ever compared.
///
/// The three arms that answer `true` without comparing anything do so for one
/// reason: none carries a start time comparable against a fresh kernel probe.
/// Treating them as mismatches would recover — and therefore respawn — on every
/// subsequent invocation, losing the crawl's page state in exactly the
/// containers that cannot supply a probe value.
const fn identity_holds(
    daemon: RecordedDaemon,
    observed: ObservedStartTime,
) -> bool {
    match daemon.start_time {
        RecordedStartTime::AbsentOrUnparseable => false,
        RecordedStartTime::Wallclock(_)
        | RecordedStartTime::WriterUnavailable => true,
        RecordedStartTime::Probe(recorded) => match observed {
            ObservedStartTime::Known(seen) => {
                recorded.abs_diff(seen) <= TOLERANCE_SECONDS
            }
            ObservedStartTime::Unavailable => true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use super::Reuse;
    use crate::executor::daemon_identity::ObservedDaemon;
    use crate::executor::daemon_identity::ObservedStartTime;
    use crate::executor::daemon_identity::RecordedDaemon;
    use crate::executor::daemon_identity::RecordedStartTime;
    use crate::executor::daemon_identity::RecordedState;

    const PID: i32 = 4242;

    fn recorded(start_time: RecordedStartTime) -> RecordedState {
        RecordedState::Daemon(RecordedDaemon {
            pid: PID,
            start_time,
        })
    }

    fn live(seconds: u64) -> ObservedDaemon {
        ObservedDaemon::Live(ObservedStartTime::Known(seconds))
    }

    fn live_unavailable() -> ObservedDaemon {
        ObservedDaemon::Live(ObservedStartTime::Unavailable)
    }

    #[test]
    fn a_matching_probe_value_reuses() {
        assert_eq!(
            evaluate(recorded(RecordedStartTime::Probe(1000)), live(1000)),
            Reuse::Reuse(PID)
        );
    }

    #[test]
    fn the_tolerance_is_one_second_in_both_directions() {
        for seen in [999, 1000, 1001] {
            assert_eq!(
                evaluate(recorded(RecordedStartTime::Probe(1000)), live(seen)),
                Reuse::Reuse(PID),
                "{seen} is within tolerance"
            );
        }
        for seen in [998, 1002] {
            assert_eq!(
                evaluate(recorded(RecordedStartTime::Probe(1000)), live(seen)),
                Reuse::Recover,
                "{seen} is outside tolerance"
            );
        }
    }

    /// The PID-recycle case: a live pid whose start time contradicts the
    /// record is not the recorded daemon.
    #[test]
    fn a_contradicted_probe_value_recovers() {
        assert_eq!(
            evaluate(recorded(RecordedStartTime::Probe(1000)), live(9999)),
            Reuse::Recover
        );
    }

    /// The three provenance-uncertain rows. Each accepts the same conservative
    /// consequence: reuse with no PID-recycle guard, rather than respawning
    /// forever.
    #[test]
    fn every_row_carrying_no_comparable_value_reuses_on_liveness_alone() {
        assert_eq!(
            evaluate(
                recorded(RecordedStartTime::Probe(1000)),
                live_unavailable()
            ),
            Reuse::Reuse(PID)
        );
        for start_time in [
            RecordedStartTime::Wallclock(1000),
            RecordedStartTime::WriterUnavailable,
        ] {
            for observed in [live(1000), live(9999), live_unavailable()] {
                assert_eq!(
                    evaluate(recorded(start_time), observed),
                    Reuse::Reuse(PID),
                    "{start_time:?} against {observed:?}"
                );
            }
        }
    }

    /// An empty recorded value bypasses the PID-recycle guard entirely, and it
    /// is precisely what a truncated write or an interrupted migration leaves
    /// behind, so it recovers rather than trusting liveness alone.
    #[test]
    fn an_absent_or_unparseable_start_time_recovers() {
        for observed in [live(1000), live_unavailable()] {
            assert_eq!(
                evaluate(
                    recorded(RecordedStartTime::AbsentOrUnparseable),
                    observed
                ),
                Reuse::Recover,
                "{observed:?}"
            );
        }
    }

    #[test]
    fn a_dead_pid_recovers_whatever_was_recorded() {
        for start_time in [
            RecordedStartTime::Probe(1000),
            RecordedStartTime::Wallclock(1000),
            RecordedStartTime::WriterUnavailable,
            RecordedStartTime::AbsentOrUnparseable,
        ] {
            assert_eq!(
                evaluate(recorded(start_time), ObservedDaemon::Absent),
                Reuse::Recover,
                "{start_time:?}"
            );
        }
    }

    #[test]
    fn an_absent_or_unusable_record_recovers_with_no_pid_to_signal() {
        for state in [RecordedState::None, RecordedState::PidUnparseable] {
            for observed in
                [live(1000), live_unavailable(), ObservedDaemon::Absent]
            {
                assert_eq!(
                    evaluate(state, observed),
                    Reuse::Recover,
                    "{state:?} against {observed:?}"
                );
            }
        }
    }

    /// Recovery is one verdict with no pid attached, so there is nothing for a
    /// caller to signal even if it wanted to. The type is what enforces this,
    /// not a convention.
    #[test]
    fn no_recovering_verdict_carries_a_pid() {
        let every_input = [
            (RecordedState::None, ObservedDaemon::Absent),
            (RecordedState::PidUnparseable, live(1000)),
            (recorded(RecordedStartTime::Probe(1000)), live(9999)),
            (recorded(RecordedStartTime::AbsentOrUnparseable), live(1000)),
            (
                recorded(RecordedStartTime::Probe(1000)),
                ObservedDaemon::Absent,
            ),
        ];
        for (state, observed) in every_input {
            assert_eq!(evaluate(state, observed), Reuse::Recover);
        }
    }
}
