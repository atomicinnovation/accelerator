//! The design context's outbound adapters.
//!
//! The module split is load-bearing: `filesystem` and `environment` are held
//! to a no-spawn rule by `cli/pup.ron`, which the process module a later
//! change adds — the Playwright launcher's daemon spawn and signalling — is
//! deliberately outside.

pub mod cue_phrases;
pub mod environment;
pub mod filesystem;

pub use cue_phrases::CompiledCuePhrases;
pub use environment::credentials_from_env;
pub use environment::named_secrets_from_env;
pub use filesystem::read_document;
pub use filesystem::DirectoryCheck;
