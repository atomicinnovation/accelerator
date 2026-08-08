//! The outbound adapters for `migrate`'s locally-declared ports: ledger and
//! manifest file I/O, the session log, VCS staleness, legacy-layout config
//! access, and interactive decision sources.

pub mod context;
pub mod corpus_index;
pub mod dirty_path_scanner;
pub mod ledger_store;
pub mod manifest_store;
pub mod merge_move;
pub mod run_lock;
pub mod session_log;
pub mod session_log_factory;
pub mod tty_decision_source;
