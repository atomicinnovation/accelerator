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
