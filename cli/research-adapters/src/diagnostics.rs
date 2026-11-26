//! Where an adapter reports a degraded but survivable condition: one line,
//! never a secret, and never a reason to fail the call.

pub use research::sources::ports::Diagnostics;

pub struct Stderr;

impl Diagnostics for Stderr {
    fn report(&self, line: &str) {
        eprintln!("{line}");
    }
}
