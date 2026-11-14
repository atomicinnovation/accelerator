//! The work-item lifecycle domain: pure decision logic for resolve,
//! next-number allocation, section-diff, tag mutation, normalisation,
//! template-hint parsing, own-identity, and raw field reads. No filesystem,
//! no subprocess, no regex — those live in the adapter/binary layers.

pub mod create;
pub mod create_batch;
pub mod dirtiness;
pub mod draft_id;
pub mod filter;
pub mod hierarchy;
pub mod id_pattern;
pub mod id_scheme;
pub mod identity;
pub mod identity_run;
pub mod inventory;
pub mod legacy_ids;
pub mod next_number;
pub mod normalise;
pub mod own_identity;
pub mod path_component;
pub mod promotion;
pub mod rekey;
pub mod requested_id;
pub mod resolve;
pub mod retirement;
pub mod section_diff;
pub mod show;
pub mod sync;
pub mod tags;
pub mod template_hints;
pub mod tracker_key;
pub mod update;
pub mod work_item_files;
