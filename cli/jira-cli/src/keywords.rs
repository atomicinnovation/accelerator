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
//!
//! `create --emit key` is the exception: it emits only the bare validated key,
//! byte-for-byte as the retiring `jira-emit-key.sh` did, and its outcome is
//! read from the exit code (`0` created, `16` created-but-unwritable).

// The keyword-surface test `#[path]`-includes this module standalone, where the
// render helpers read as unused; they are used from `main`.
#![allow(dead_code)]

use serde_json::Value;

/// Emits the trailing `<keyword>\t<detail>` discriminant for a text subcommand.
pub fn text_line(keyword: &str, detail: &str) {
    println!("{keyword}\t{detail}");
}

/// Embeds the discriminant as a top-level `outcome` field in a JSON envelope,
/// leaving every existing path (the skill bodies read `.fields…`) intact.
#[must_use]
pub fn with_outcome(mut envelope: Value, keyword: &str) -> Value {
    if let Value::Object(map) = &mut envelope {
        map.insert("outcome".to_owned(), Value::String(keyword.to_owned()));
    }
    envelope
}

macro_rules! keyword_enum {
    ($name:ident { $($variant:ident => $keyword:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            #[must_use]
            pub const fn keyword(self) -> &'static str {
                match self {
                    $(Self::$variant => $keyword),+
                }
            }
        }
    };
}

keyword_enum!(Show { Found => "found", NotFound => "not-found" });
keyword_enum!(Search { Results => "results", Empty => "empty" });
keyword_enum!(Create { Created => "created" });
keyword_enum!(Update { Updated => "updated" });
keyword_enum!(Comment {
    Added => "added",
    Listed => "listed",
    Edited => "edited",
    Deleted => "deleted",
});
keyword_enum!(Transition { Transitioned => "transitioned" });
keyword_enum!(Init {
    Verified => "verified",
    Discovered => "discovered",
    ProjectsListed => "projects-listed",
    FieldsListed => "fields-listed",
    DefaultPrompted => "default-prompted",
});
keyword_enum!(Fields {
    Refreshed => "refreshed",
    Resolved => "resolved",
    Listed => "listed",
});
