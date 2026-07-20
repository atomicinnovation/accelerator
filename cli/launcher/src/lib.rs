//! The accelerator CLI library crate. Each built-in subcommand is realised as a
//! hexagon under its own module; the binary entry point (`main.rs`) is the
//! composition root that wires the concrete adapters to the ports and
//! dispatches the parsed command.

pub mod config_command;
pub mod launch;
pub mod version;
