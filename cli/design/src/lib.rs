//! The design bounded context: inspecting a running design surface, auditing
//! the documents produced from it, and knowing whether the runtime to do
//! either is available here.
//!
//! Pure logic only. Every filesystem read, environment read, process spawn and
//! compiled regex lives behind a port implemented in `design-adapters`.

pub mod access_policy;
pub mod credentials;
pub mod cue_phrase_audit;
pub mod host;
pub mod host_reach;
pub mod leaked_credentials;
pub mod runtime;
pub mod source_location;
pub mod verdict;

pub use crate::access_policy::evaluate;
pub use crate::access_policy::Allowances;
pub use crate::credentials::resolve;
pub use crate::credentials::AuthMode;
pub use crate::credentials::Credentials;
pub use crate::cue_phrase_audit::audit;
pub use crate::cue_phrase_audit::CuePhraseMatcher;
pub use crate::cue_phrase_audit::CUE_PHRASE_PATTERNS;
pub use crate::host::Host;
pub use crate::host::HostError;
pub use crate::host_reach::HostReach;
pub use crate::leaked_credentials::scan;
pub use crate::leaked_credentials::NamedSecret;
pub use crate::runtime::downgrade::DowngradeReason;
pub use crate::source_location::SourceLocation;
pub use crate::verdict::Verdict;
