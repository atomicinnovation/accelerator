//! The closed, per-subcommand outcome keyword, carried as a top-level
//! `outcome` field in each JSON envelope. The skills branch on it, not on
//! the exit code.

// The keyword-surface test `#[path]`-includes this module standalone, where
// the render helper reads as unused.
#![allow(dead_code)]

use serde_json::Value;

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
    CapHit,
}

impl Search {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Results => "results",
            Self::Empty => "empty",
            Self::Truncated => "truncated",
            Self::CapHit => "cap-hit",
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
