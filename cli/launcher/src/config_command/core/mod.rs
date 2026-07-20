//! The `config` command's core: the composed port bundle the inbound adapter
//! reads through, and the shared failure policy.
//!
//! The bundle is expressed in `config`-crate traits, never the concrete
//! adapter, so no launcher module outside the composition root names
//! `config_adapters`.

use config::ConfigAccess;

/// How a read failure is surfaced at a splice-safe call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFailure {
    /// Exit non-zero with the error on stderr.
    Fail,
    /// Exit zero, degrading per the subcommand's fail-safe contract. For a
    /// caller that splices this command's stdout into a prompt: a non-zero
    /// exit there discards the whole prompt, so failing loudly would disable
    /// the caller rather than inform it.
    Degrade,
}

/// The composed configuration ports, handed to the inbound adapter by the
/// composition root. Grows a driven-port field per subcommand family as they
/// land; for now it exposes the resolution service alone.
pub struct ConfigStack {
    service: Box<dyn ConfigAccess>,
}

impl ConfigStack {
    #[must_use]
    pub fn new(service: Box<dyn ConfigAccess>) -> Self {
        Self { service }
    }

    #[must_use]
    pub fn config(&self) -> &dyn ConfigAccess {
        self.service.as_ref()
    }
}
