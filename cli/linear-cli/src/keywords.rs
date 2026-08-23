//! The closed, per-subcommand keyword discriminant (Decision 11).
//!
//! Each outcome is a typed enum with a `keyword(self)` projection, so
//! exhaustiveness is compiler-checked and a new variant cannot compile without
//! a keyword. The repointed skill bodies branch on the keyword, not the exit
//! integer. The carrier depends on the subcommand's stdout shape:
//!
//! - text-emitting subcommands emit a trailing `<keyword>\t<detail>` line;
//! - JSON-emitting subcommands carry a top-level `outcome` field inside the
//!   envelope, so stdout stays one parseable document.

// The keyword-surface test `#[path]`-includes this module standalone, where the
// render helpers read as unused; they are used from `main`.
#![allow(dead_code)]

use serde_json::Value;

/// Emits the trailing `<keyword>\t<detail>` discriminant for a text subcommand.
pub fn text_line(keyword: &str, detail: &str) {
    println!("{keyword}\t{detail}");
}

/// Embeds the discriminant as a top-level `outcome` field in a JSON envelope,
/// leaving every existing path (the skill bodies read `.data…`) intact.
#[must_use]
pub fn with_outcome(mut envelope: Value, keyword: &str) -> Value {
    if let Value::Object(map) = &mut envelope {
        map.insert("outcome".to_owned(), Value::String(keyword.to_owned()));
    }
    envelope
}

#[derive(Debug, Clone, Copy)]
pub enum Show {
    Found,
    NotFound,
}

impl Show {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::NotFound => "not-found",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Search {
    Results,
    Empty,
    Truncated,
}

impl Search {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Results => "results",
            Self::Empty => "empty",
            Self::Truncated => "truncated",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Create {
    Created,
    WritebackFailed,
}

impl Create {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::WritebackFailed => "writeback-failed",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Update {
    Updated,
}

impl Update {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Updated => "updated",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Comment {
    Added,
}

impl Comment {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Added => "added",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Transition {
    Transitioned,
}

impl Transition {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Transitioned => "transitioned",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Attach {
    Attached,
}

impl Attach {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Attached => "attached",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Init {
    Verified,
    Listed,
    Discovered,
}

impl Init {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Listed => "listed",
            Self::Discovered => "discovered",
        }
    }
}
