//! Persistence and apply for the remote sync engine: digest computation,
//! the baseline document, and the per-item apply sequence over
//! `tracker::RemoteTracker`.

pub mod apply;
mod attempt;
pub mod baseline;
pub mod baseline_digester;
pub mod baseline_store;
pub mod body_comparison;
pub mod create;
pub mod created_baseline;
pub mod digest;
pub mod fetch;
pub mod files;
pub mod identity_settlement;
pub mod item_snapshot;
pub mod kept_edits;
pub mod operation;
mod ordering;
pub mod pending_push;
pub mod realign;
pub mod recovery_label;
pub mod run;
pub mod scope;
pub mod send_refusal;
pub mod settled_run;
pub mod synced_fields;
pub mod working_copy_status;
