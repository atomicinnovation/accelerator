//! The work-item lifecycle domain: pure decision logic for resolve,
//! next-number allocation, section-diff, tag mutation, normalisation,
//! template-hint parsing, own-identity, and raw field reads. No filesystem,
//! no subprocess, no regex — those live in the adapter/binary layers.

pub mod create;
pub mod draft_id;
pub mod filter;
pub mod identity;
pub mod next_number;
pub mod normalise;
pub mod own_identity;
pub mod resolve;
pub mod section_diff;
pub mod show;
pub mod sync;
pub mod tags;
pub mod template_hints;
pub mod update;
pub mod work_item_files;
