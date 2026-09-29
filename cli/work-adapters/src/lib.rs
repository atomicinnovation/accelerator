//! Outbound adapters for the `work` domain crate: filesystem reads,
//! in-process section diffing, and VCS-derived authorship.

pub mod author;
pub mod create_request_fields;
pub mod diff;
pub mod draft_id;
pub mod filesystem;
pub mod promotion;
pub mod promotion_records;
pub mod remote_create;
pub mod retirement;
pub mod retirement_records;
pub mod sync;
