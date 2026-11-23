//! The queue engine every queued service shares: a call joins the service's
//! queue, waits its turn, and is served by its family's own requests,
//! retried on the schedule within the call's deadline.

use std::time::Duration;
use std::time::SystemTime;

use crate::sources::admission::admitted;
use crate::sources::classify::FetchError;
use crate::sources::classify::Reason;
use crate::sources::classify::Response;
use crate::sources::classify::Verdict;
use crate::sources::ports::Attempted;
use crate::sources::ports::Clock;
use crate::sources::ports::Contention;
use crate::sources::ports::ContentionLog;
use crate::sources::ports::OutOfBudget;
use crate::sources::ports::PacingGate;
use crate::sources::ports::ServingTurn;
use crate::sources::ports::Transport;
use crate::sources::ports::Unavailable;
use crate::sources::queue::Binding;
use crate::sources::queue::Joining;
use crate::sources::queue::Position;
use crate::sources::queue::ServiceQueue;
use crate::sources::queue::Ticket;
use crate::sources::queue::TicketRejection;
use crate::sources::request::UpstreamRequest;
use crate::sources::schedule::Deadline;
use crate::sources::schedule::RetryNumber;
use crate::sources::schedule::RetrySchedule;

/// How a queued call ended, whatever its family serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Queued<T> {
    Served(T),
    /// The upstream or the queue left nothing to serve.
    Unavailable(Unavailable),
    /// The call ran out of budget but kept its place; re-presenting the
    /// ticket resumes it.
    Waiting {
        ticket: Ticket,
        position: Position,
    },
    Rejected(TicketRejection),
}

pub(crate) struct QueuePorts<'a> {
    pub transport: &'a dyn Transport,
    pub clock: &'a dyn Clock,
    pub gate: &'a dyn PacingGate,
    pub queue: &'a dyn ServiceQueue,
    pub contention: &'a dyn ContentionLog,
}

/// What a family's turn produces: its answer, an `Unavailable` the engine
/// reports as is, or a budget spent before it could finish.
pub(crate) type Served<T> = Result<Result<T, Unavailable>, OutOfBudget>;

/// Queues `binding`'s call and serves it with `serve` once its turn comes.
///
/// A call that runs out of budget keeps its place and is
/// [`Queued::Waiting`]; one that never got a place reports its last upstream
/// failure or lock contention instead.
pub(crate) fn queued<T>(
    binding: &Binding,
    presented: Option<&Ticket>,
    ports: &QueuePorts<'_>,
    deadline: &Deadline,
    serve: &mut dyn FnMut(&Attempts<'_>) -> Served<T>,
) -> Queued<T> {
    match ports.queue.join(binding, presented, deadline) {
        Joining::Rejected(rejection) => Queued::Rejected(rejection),
        Joining::OverCap {
            ticket,
            last_retryable,
        } => upstream_reason_or_contention(
            ports,
            last_retryable,
            Contention::TicketPastCap(ticket),
        ),
        Joining::Unqueued => {
            match served_when(&|| true, ports, deadline, serve) {
                Ok(served) => folded(served),
                Err(out_of_budget) => upstream_reason_or_contention(
                    ports,
                    out_of_budget.last_retryable,
                    Contention::QueueUnusable,
                ),
            }
        }
        Joining::Unheld { ticket, position } => {
            served_when(&|| true, ports, deadline, serve)
                .map_or(Queued::Waiting { ticket, position }, folded)
        }
        Joining::Queued(place) => {
            match served_when(&|| place.is_front(), ports, deadline, serve) {
                Ok(served) => {
                    place.leave(deadline);
                    folded(served)
                }
                Err(out_of_budget) => {
                    let ticket = place.ticket().clone();
                    let position = place
                        .step_aside(out_of_budget.last_retryable, deadline);
                    Queued::Waiting { ticket, position }
                }
            }
        }
    }
}

fn folded<T>(served: Result<T, Unavailable>) -> Queued<T> {
    served.map_or_else(Queued::Unavailable, Queued::Served)
}

/// The only place the engine records contention: a call that ended with no
/// upstream failure to report instead.
fn upstream_reason_or_contention<T>(
    ports: &QueuePorts<'_>,
    last_retryable: Option<Reason>,
    contention: Contention,
) -> Queued<T> {
    Queued::Unavailable(last_retryable.map_or_else(
        || {
            ports.contention.record(contention);
            Unavailable::lock_contention()
        },
        Unavailable::new,
    ))
}

fn served_when<T>(
    ready: &dyn Fn() -> bool,
    ports: &QueuePorts<'_>,
    deadline: &Deadline,
    serve: &mut dyn FnMut(&Attempts<'_>) -> Served<T>,
) -> Result<Result<T, Unavailable>, OutOfBudget> {
    let turn = admitted(ports.gate, ports.clock, deadline, ready)?;
    let attempts = Attempts::within(
        turn.as_ref(),
        ports.transport,
        ports.clock,
        ports.gate,
        deadline,
    );
    let served = serve(&attempts);
    drop(turn);
    served
}

pub(crate) enum Settled<T> {
    Delivered(T),
    Empty,
    Unavailable(Unavailable),
    Failed(FetchError),
}

enum Step<T> {
    Settled(Settled<T>),
    Retry {
        reason: Reason,
        retry_after: Option<Duration>,
    },
}

/// The attempts one turn makes, every one paced by the turn's gate.
pub(crate) struct Attempts<'a> {
    transport: &'a dyn Transport,
    clock: &'a dyn Clock,
    turn: &'a dyn ServingTurn,
    interval: Duration,
    deadline: &'a Deadline,
}

impl<'a> Attempts<'a> {
    pub(crate) fn within(
        turn: &'a dyn ServingTurn,
        transport: &'a dyn Transport,
        clock: &'a dyn Clock,
        gate: &dyn PacingGate,
        deadline: &'a Deadline,
    ) -> Self {
        Self {
            transport,
            clock,
            turn,
            interval: gate.pacing().interval(),
            deadline,
        }
    }

    /// Attempts `request` until it settles or the retries run out, the last
    /// retryable attempt then deciding the reason. Each attempt must leave
    /// room for a serving window after its wait, or the call is out of
    /// budget. A delivered body is accepted inside the turn.
    pub(crate) fn until_settled<T>(
        &self,
        request: &UpstreamRequest,
        classify: &dyn Fn(Response) -> Verdict,
        accept: &mut dyn FnMut(Vec<u8>) -> Result<T, FetchError>,
    ) -> Result<Settled<T>, OutOfBudget> {
        let mut next_retry = RetryNumber::first();
        let mut wait = Duration::ZERO;
        let mut last_retryable = None;
        loop {
            let serving_window = wait + self.interval;
            if !self
                .deadline
                .admits_attempt_after(self.clock.now(), serving_window)
            {
                return Err(OutOfBudget { last_retryable });
            }
            if !wait.is_zero() {
                self.clock.sleep(wait);
            }

            let mut step = None;
            let passed = self.turn.paced(
                &mut || {
                    let verdict = classify(self.transport.send(request));
                    let defer_until = self.deferral(&verdict, next_retry);
                    step = Some(Self::step(verdict, accept));
                    Attempted { defer_until }
                },
                self.deadline,
            );
            if passed.is_err() {
                return Err(OutOfBudget { last_retryable });
            }

            match step {
                Some(Step::Settled(settled)) => return Ok(settled),
                Some(Step::Retry {
                    reason,
                    retry_after,
                }) => {
                    last_retryable = Some(reason);
                    let Some(next_wait) =
                        RetrySchedule::wait_before(next_retry, retry_after)
                    else {
                        return Ok(Settled::Unavailable(Unavailable::new(
                            reason,
                        )));
                    };
                    wait = next_wait;
                    next_retry = next_retry.next();
                }
                None => return Err(OutOfBudget { last_retryable }),
            }
        }
    }

    fn deferral(
        &self,
        verdict: &Verdict,
        next_retry: RetryNumber,
    ) -> Option<SystemTime> {
        let Verdict::Retry { retry_after, .. } = verdict else {
            return None;
        };
        verdict.is_throttling().then(|| {
            self.clock.wall_now()
                + RetrySchedule::deferral_after(next_retry, *retry_after)
        })
    }

    fn step<T>(
        verdict: Verdict,
        accept: &mut dyn FnMut(Vec<u8>) -> Result<T, FetchError>,
    ) -> Step<T> {
        Step::Settled(match verdict {
            Verdict::Deliver(body) => {
                accept(body).map_or_else(Settled::Failed, Settled::Delivered)
            }
            Verdict::Empty => Settled::Empty,
            Verdict::Unavailable(reason) => {
                Settled::Unavailable(Unavailable::new(reason))
            }
            Verdict::Fail(error) => Settled::Failed(error),
            Verdict::Retry {
                reason,
                retry_after,
            } => {
                return Step::Retry {
                    reason,
                    retry_after,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_engine_depends_on_no_fetch_workflow() {
        assert!(
            !include_str!("serving.rs").contains(concat!("sources::", "fetch"))
        );
    }
}
