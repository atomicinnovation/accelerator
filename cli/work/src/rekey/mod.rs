//! Re-keying: converging every work item's `id` onto the current
//! `work.id_pattern`.

pub mod collision;
pub mod creation_date;
pub mod entry;
pub mod fingerprint;
pub mod folding;
pub mod mapping;
pub mod outcome;
pub mod partly_applied;
pub mod run_plan;
pub mod write_set;
