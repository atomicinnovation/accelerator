//! How far apart a service's requests must be, and from which earlier
//! moment that spacing runs.

use std::num::NonZeroU8;
use std::time::Duration;

const ONE_SLOT: NonZeroU8 = NonZeroU8::MIN;
const FASTEST_ADMISSION_POLL: Duration = Duration::from_millis(10);
const SLOWEST_ADMISSION_POLL: Duration = Duration::from_millis(100);

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

    /// How many calls may be served at once.
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
