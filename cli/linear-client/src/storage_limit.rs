//! What Linear cannot store as text, however it is escaped.
//!
//! Linear decodes an escaped `<` or `&` before storing it, and splits a
//! table cell at any `|`, so nothing sent can avoid such a rewrite.

/// A character Linear never stores as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageLimit {
    AngleBracket,
    Ampersand,
    TableCellPipe,
}

impl StorageLimit {
    pub const ALL: [Self; 3] =
        [Self::AngleBracket, Self::Ampersand, Self::TableCellPipe];

    /// The limits a line of local Markdown meets, each once, in a fixed
    /// order.
    #[must_use]
    pub fn in_sent(sent: &str) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|limit| limit.met_by(sent))
            .collect()
    }

    #[must_use]
    pub const fn character(self) -> char {
        match self {
            Self::AngleBracket => '<',
            Self::Ampersand => '&',
            Self::TableCellPipe => '|',
        }
    }

    fn met_by(self, sent: &str) -> bool {
        const ESCAPED_AMPERSANDS: [&str; 4] =
            ["\\&", "&amp;", "&#38;", "&#x26;"];
        let lowered = sent.to_ascii_lowercase();
        let holds = |text: &str, forms: &[&str]| {
            forms.iter().any(|form| text.contains(form))
        };
        match self {
            Self::AngleBracket => holds(
                &ESCAPED_AMPERSANDS
                    .iter()
                    .fold(lowered, |text, form| text.replace(form, "")),
                &["\\<", "&lt;", "&#60;", "&#x3c;"],
            ),
            Self::Ampersand => holds(&lowered, &ESCAPED_AMPERSANDS),
            Self::TableCellPipe => {
                sent.trim_start().starts_with('|')
                    && holds(&lowered, &["\\|", "&#124;", "&#x7c;"])
            }
        }
    }
}
