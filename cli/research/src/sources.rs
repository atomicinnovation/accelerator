//! Asking OpenAlex and arXiv for works: the request a caller may make, the
//! retried fetch workflow over its ports, and the normalised, tiered record
//! each work becomes.

pub mod arxiv;
pub mod classify;
pub mod fetch;
pub mod openalex;
pub mod queue;
pub mod record;
pub mod request;
pub mod schedule;
pub mod tier;
