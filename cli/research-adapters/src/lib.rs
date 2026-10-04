//! The research context's adapters: the HTTP transport, the per-source
//! decoders, the clock, and the pacing gates, each implementing a port the
//! `research` domain declares, and the reader of a topic-research set's round
//! inputs.

pub mod arxiv_xml;
pub mod clock;
pub mod conduct;
pub mod confirmations;
pub mod diagnostics;
pub mod openalex_json;
pub mod pacing;
pub mod scratch;
pub mod topic;
pub mod transport;
pub mod unicode_text;
