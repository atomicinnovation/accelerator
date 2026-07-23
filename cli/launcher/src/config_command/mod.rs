//! The `config` subcommand's hexagon.
//!
//! `core` holds the composed port bundle and view assembly; `inbound` maps a
//! parsed request onto the core and presents the result. The domain resolution
//! lives in the `config` crate; this module names neither `config_adapters` nor
//! any concrete adapter — the composition root injects the ports.

pub mod core;
pub mod inbound;
pub mod render;
