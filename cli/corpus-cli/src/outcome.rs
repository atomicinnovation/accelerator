//! The shared command-layer output shape. `main.rs` prints it verbatim and
//! maps `Ok`/`Err` to an exit code; command modules never print directly.

/// A command's rendered output.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
}
