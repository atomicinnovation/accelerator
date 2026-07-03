//! The `version` subcommand hexagon.
//!
//! The domain core and its two ports (`core`), the clap inbound/driving adapter
//! (`inbound`), and the vergen outbound/driven adapter (`outbound`). The core
//! depends inward only — cargo-pup enforces that direction.

pub mod core;
pub mod inbound;
pub mod outbound;
