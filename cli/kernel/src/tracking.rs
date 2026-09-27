//! The answer `accelerator vcs tracking` prints, shared so the sub-binary
//! that renders it and the launcher that parses it cannot drift apart.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

/// Whether the VCS tracks a file. A sub-binary that cannot tell prints
/// neither answer and exits non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingAnswer {
    Tracked,
    Untracked,
}

impl TrackingAnswer {
    const fn word(self) -> &'static str {
        match self {
            Self::Tracked => "tracked",
            Self::Untracked => "untracked",
        }
    }
}

impl Display for TrackingAnswer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.word())
    }
}

/// Printed output that is neither answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnrecognisedTrackingAnswer;

impl Display for UnrecognisedTrackingAnswer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a tracking answer")
    }
}

impl std::error::Error for UnrecognisedTrackingAnswer {}

impl FromStr for TrackingAnswer {
    type Err = UnrecognisedTrackingAnswer;

    fn from_str(printed: &str) -> Result<Self, Self::Err> {
        let word = printed.strip_suffix('\n').unwrap_or(printed);
        [Self::Tracked, Self::Untracked]
            .into_iter()
            .find(|answer| answer.word() == word)
            .ok_or(UnrecognisedTrackingAnswer)
    }
}

#[cfg(test)]
mod tests {
    use super::TrackingAnswer;

    #[test]
    fn each_answer_parses_back_from_its_rendering() {
        for answer in [TrackingAnswer::Tracked, TrackingAnswer::Untracked] {
            assert_eq!(answer.to_string().parse(), Ok(answer));
        }
    }

    #[test]
    fn the_answers_render_as_single_words() {
        assert_eq!(TrackingAnswer::Tracked.to_string(), "tracked");
        assert_eq!(TrackingAnswer::Untracked.to_string(), "untracked");
    }

    #[test]
    fn a_trailing_newline_is_part_of_a_printed_answer() {
        assert_eq!("untracked\n".parse(), Ok(TrackingAnswer::Untracked));
    }

    #[test]
    fn anything_else_is_unrecognised() {
        for text in ["", "Tracked", "unknown", "tracked untracked", " tracked"]
        {
            assert!(
                text.parse::<TrackingAnswer>().is_err(),
                "{text:?} must not parse"
            );
        }
    }
}
