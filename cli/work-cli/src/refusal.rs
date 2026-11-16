//! A command's refusal: the line it prints on stderr and the code it exits
//! with.

use std::process::ExitCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub message: String,
    pub code: u8,
}

impl Refusal {
    /// Prints the refusal's line on stderr and gives its exit code.
    #[must_use]
    pub fn report(&self) -> ExitCode {
        eprintln!("{}", self.message);
        ExitCode::from(self.code)
    }
}
