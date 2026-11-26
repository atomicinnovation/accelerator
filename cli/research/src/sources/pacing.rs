//! How far apart a service's requests must be, from which earlier moment
//! that spacing runs, and the shared state every caller reserves and
//! confirms its send against.

use std::num::NonZeroU8;
use std::time::Duration;
use std::time::SystemTime;

const ONE_SLOT: NonZeroU8 = NonZeroU8::MIN;
const FASTEST_ADMISSION_POLL: Duration = Duration::from_millis(10);
const SLOWEST_ADMISSION_POLL: Duration = Duration::from_millis(100);
/// A send is stamped before its request leaves, so its arrival at a
/// finish-spaced source is unobserved and can trail the stamp by more than
/// the next request's does.
const SEND_MARGIN: Duration = Duration::from_millis(100);
/// No source is ever asked to wait longer, so a deferral further ahead can
/// only come from a wall-clock step.
const DEFERRAL_CEILING: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacedFrom {
    /// Spaced from when the source certainly saw the previous request, so
    /// only one call can be served at a time.
    PreviousFinish,
    /// Spaced from the previous send, with up to `slots` calls in flight.
    PreviousStart { slots: NonZeroU8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pacing {
    interval: Duration,
    spaced_from: SpacedFrom,
}

impl Pacing {
    pub const UNPACED: Self = Self::previous_finish(Duration::ZERO);

    pub const fn previous_finish(interval: Duration) -> Self {
        Self {
            interval,
            spaced_from: SpacedFrom::PreviousFinish,
        }
    }

    pub const fn previous_start(interval: Duration, slots: NonZeroU8) -> Self {
        Self {
            interval,
            spaced_from: SpacedFrom::PreviousStart { slots },
        }
    }

    pub const fn interval(&self) -> Duration {
        self.interval
    }

    pub const fn spaced_from(&self) -> SpacedFrom {
        self.spaced_from
    }

    pub const fn slots(&self) -> NonZeroU8 {
        match self.spaced_from {
            SpacedFrom::PreviousFinish => ONE_SLOT,
            SpacedFrom::PreviousStart { slots } => slots,
        }
    }

    /// How often a call waiting for admission looks again: often enough to
    /// take a free slot well within one interval, never so often it spins.
    pub fn admission_poll(&self) -> Duration {
        let quarter = self.interval / 4;
        let whole_millis = Duration::from_millis(
            u64::try_from(quarter.as_millis()).unwrap_or(u64::MAX),
        );
        whole_millis.clamp(FASTEST_ADMISSION_POLL, SLOWEST_ADMISSION_POLL)
    }
}

/// What every caller of one service reads and writes before it sends.
///
/// `last_sent` is only ever an actual send; `next_free` is the reservation
/// cursor, used only when spacing runs from the previous start.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PacingState {
    pub last_sent: Option<SystemTime>,
    pub next_free: Option<SystemTime>,
    pub last_finish: Option<SystemTime>,
    pub not_before: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    send_at: SystemTime,
}

impl Reservation {
    pub const fn send_at(&self) -> SystemTime {
        self.send_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    Send(PacingState),
    /// Spacing or a deferral still forbids sending; the reservation stands.
    WaitUntil(SystemTime),
    /// A deferral began after the reservation, which is already handed back.
    Deferred(PacingState),
}

impl PacingState {
    #[must_use]
    pub fn reserve(
        self,
        pacing: &Pacing,
        now: SystemTime,
    ) -> (Reservation, Self) {
        match pacing.spaced_from {
            SpacedFrom::PreviousFinish => (
                Reservation {
                    send_at: self.finish_spaced_send(pacing, now).max(now),
                },
                self,
            ),
            SpacedFrom::PreviousStart { .. } => {
                let send_at = whole_millis(
                    [self.next_free, self.start_spaced_send(pacing)]
                        .into_iter()
                        .flatten()
                        .fold(now, SystemTime::max),
                );
                (
                    Reservation { send_at },
                    Self {
                        next_free: Some(send_at + pacing.interval),
                        ..self
                    },
                )
            }
        }
    }

    /// Never reads `next_free`, so no caller is held back by its own or a
    /// later reservation.
    #[must_use]
    pub fn confirm(
        self,
        pacing: &Pacing,
        reservation: &Reservation,
        now: SystemTime,
    ) -> Confirmation {
        let deferred_since_reserving = self
            .not_before
            .is_some_and(|until| until > now && until > reservation.send_at);
        if deferred_since_reserving {
            return Confirmation::Deferred(self.release(pacing, reservation));
        }
        let earliest = match pacing.spaced_from {
            SpacedFrom::PreviousFinish => self.finish_spaced_send(pacing, now),
            SpacedFrom::PreviousStart { .. } => self
                .start_spaced_send(pacing)
                .unwrap_or(SystemTime::UNIX_EPOCH),
        };
        if earliest > now {
            return Confirmation::WaitUntil(earliest);
        }
        Confirmation::Send(Self {
            last_sent: Some(now),
            ..self
        })
    }

    /// Hands a reservation back, which only frees its time while no later
    /// reservation has been made.
    #[must_use]
    pub fn release(self, pacing: &Pacing, reservation: &Reservation) -> Self {
        let latest =
            self.next_free == Some(reservation.send_at + pacing.interval);
        match pacing.spaced_from {
            SpacedFrom::PreviousStart { .. } if latest => Self {
                next_free: Some(reservation.send_at),
                ..self
            },
            _ => self,
        }
    }

    #[must_use]
    pub const fn finish(self, at: SystemTime) -> Self {
        Self {
            last_finish: Some(at),
            ..self
        }
    }

    /// Deferrals only move forward, so a shorter one never cuts short a
    /// longer one another caller is still honouring.
    #[must_use]
    pub fn defer(self, until: SystemTime, now: SystemTime) -> Self {
        let latest = self
            .not_before
            .map_or(until, |existing| existing.max(until));
        Self {
            not_before: Some(latest.min(now + DEFERRAL_CEILING)),
            ..self
        }
    }

    /// Repairs what only a wall-clock step or a corrupt file can produce,
    /// and leaves every other state exactly as it is.
    #[must_use]
    pub fn trust(self, pacing: &Pacing, now: SystemTime) -> Self {
        let not_before = self
            .not_before
            .filter(|not_before| *not_before <= now + DEFERRAL_CEILING);
        let stamped_ahead =
            |at: Option<SystemTime>| at.is_some_and(|at| at > now);
        let clock_stepped_back =
            stamped_ahead(self.last_sent) || stamped_ahead(self.last_finish);
        let (last_sent, last_finish) = if clock_stepped_back {
            let finish_preceded_send = self.last_finish < self.last_sent;
            (
                self.last_sent.map(|sent| sent.min(now)),
                self.last_finish
                    .filter(|_| !finish_preceded_send)
                    .map(|finish| finish.min(now)),
            )
        } else {
            (self.last_sent, self.last_finish)
        };
        let trusted = Self {
            last_sent,
            next_free: self.next_free,
            last_finish,
            not_before,
        };
        Self {
            next_free: trusted.bounded_cursor(pacing, now),
            ..trusted
        }
    }

    /// The cursor, clamped so a clock step cannot wedge the pool: no call
    /// can rightly be reserved beyond one interval per slot past the next
    /// send spacing allows.
    fn bounded_cursor(
        &self,
        pacing: &Pacing,
        now: SystemTime,
    ) -> Option<SystemTime> {
        let SpacedFrom::PreviousStart { slots } = pacing.spaced_from else {
            return self.next_free;
        };
        let base = [self.not_before, self.start_spaced_send(pacing)]
            .into_iter()
            .flatten()
            .fold(now, SystemTime::max);
        let bound = base + pacing.interval * u32::from(slots.get());
        self.next_free.map(|next_free| next_free.min(bound))
    }

    fn start_spaced_send(&self, pacing: &Pacing) -> Option<SystemTime> {
        let spaced = self.last_sent.map(|sent| sent + pacing.interval);
        spaced.max(self.not_before)
    }

    /// A send with no finish after it, from a call killed mid-request, is
    /// spaced from the send instead, with a margin for its unseen arrival.
    fn finish_spaced_send(
        &self,
        pacing: &Pacing,
        now: SystemTime,
    ) -> SystemTime {
        let spaced = match (self.last_sent, self.last_finish) {
            (Some(sent), finish) if finish < Some(sent) => {
                Some(sent + pacing.interval + SEND_MARGIN)
            }
            (_, finish) => finish.map(|finish| finish + pacing.interval),
        };
        [spaced, self.not_before]
            .into_iter()
            .flatten()
            .fold(now, SystemTime::max)
    }
}

fn whole_millis(at: SystemTime) -> SystemTime {
    let since_epoch = at
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    SystemTime::UNIX_EPOCH
        + Duration::from_millis(
            u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX),
        )
}
