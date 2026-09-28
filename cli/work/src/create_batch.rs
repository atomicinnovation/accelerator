//! What became of each entry of a batch create, as `work create-batch`
//! reports it.

use crate::sync::PushOutcome;

const AWAITING_HUMAN: u8 = 4;

/// What became of one entry, as the batch reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchKeyword {
    Pushed(PushOutcome),
    /// Created without a push: a draft, or under a numeric pattern an
    /// unsynced item.
    Declined,
    /// An earlier create of the same content is still awaiting promotion.
    Pending,
}

impl BatchKeyword {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Pushed(outcome) => outcome.keyword(),
            Self::Declined => "declined",
            Self::Pending => "pending",
        }
    }

    /// The code this entry alone would exit the batch with.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Pushed(outcome) => outcome.exit_code(),
            Self::Declined => 0,
            Self::Pending => AWAITING_HUMAN,
        }
    }
}
