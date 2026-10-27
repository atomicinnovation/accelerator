//! The research context's adapters: the HTTP transport, the per-source
//! decoders, the clock, the pacing gates and the fetch queue, each
//! implementing a port the `research` domain declares, and the reader of a
//! topic-research set's round inputs.

pub mod arxiv_xml;
pub mod bounded;
pub mod clock;
pub mod conduct;
pub mod confirmations;
pub mod contention;
pub mod diagnostics;
pub mod host_form;
pub mod host_resolver;
pub mod openalex_json;
pub mod pacing;
pub mod page_transport;
pub mod queue;
pub mod scratch;
pub mod topic;
pub mod transport;
pub mod unicode_text;
pub mod web_html;
