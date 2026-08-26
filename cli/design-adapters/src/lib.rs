//! The design context's outbound adapters.
//!
//! The module split is load-bearing: `filesystem` and `environment` are held
//! to a no-spawn rule by `cli/pup.ron`, which `process` — the Playwright
//! launcher's daemon spawn, signalling and process-image replacement — is
//! deliberately outside.

pub mod clock;
pub mod cue_phrases;
pub mod ensure;
pub mod environment;
pub mod filesystem;
pub mod lock;
pub mod marker;
pub mod paths;
pub mod platform;
pub mod process;
pub mod state;

pub use clock::MonotonicClock;
pub use cue_phrases::CompiledCuePhrases;
pub use environment::credentials_from_env;
pub use environment::named_secrets_from_env;
pub use filesystem::read_document;
pub use filesystem::DirectoryCheck;
pub use lock::FileLock;
pub use paths::HostPaths;
pub use process::BootstrapLog;
pub use process::DaemonSpawner;
pub use process::ExecClient;
pub use process::HostControl;
pub use process::HostProbe;
pub use state::StateDirectory;
