//! Asking OpenAlex and arXiv for works: the request a caller may make, the
//! retried fetch workflow over its ports, and the normalised, tiered record
//! each work becomes.

mod admission;
pub mod arxiv;
pub mod classify;
pub mod crossref;
pub mod fetch;
pub mod metadata;
pub mod openalex;
pub mod pacing;
pub mod ports;
pub mod queue;
pub mod record;
pub mod request;
pub mod schedule;
pub mod service;
pub mod serving;
pub mod tier;
pub mod web;
