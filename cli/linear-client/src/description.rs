//! An issue description as Linear receives it.

use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Description<'a> {
    /// Escaped on send, so Linear's parser reads it as `CommonMark` does.
    Markdown(&'a str),
    /// For capturing what Linear's parser does to an unescaped body.
    Verbatim(&'a str),
}

impl<'a> Description<'a> {
    pub(crate) fn wire(self) -> Cow<'a, str> {
        match self {
            Self::Markdown(markdown) => {
                Cow::Owned(markdown::linear::escape_for_push(markdown))
            }
            Self::Verbatim(text) => Cow::Borrowed(text),
        }
    }
}
