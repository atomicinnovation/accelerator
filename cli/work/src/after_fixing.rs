//! What a corpus problem tells the user to do once its fix is made.

/// What follows a corpus problem's fix: re-running the command it
/// refused, or nothing more when the command went ahead with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterFixing {
    RerunThisCommand,
    NothingMore,
}

impl AfterFixing {
    pub(crate) const fn then(self) -> &'static str {
        match self {
            Self::RerunThisCommand => ", then re-run this command",
            Self::NothingMore => "",
        }
    }
}
