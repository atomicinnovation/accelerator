//! The `config` command's core: the composed port bundle the inbound adapter
//! reads through, and the shared failure policy.
//!
//! The bundle is expressed in `config`-crate traits, never the concrete
//! adapter, so no launcher module outside the composition root names
//! `config_adapters`.

pub mod agents;
pub mod context;

use config::{ConfigAccess, ReadConfigLevel, ReadContent};

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

/// The composed configuration ports, handed to the inbound adapter.
///
/// The composition root supplies the resolution service for scalar reads and
/// the raw level reader the view-assembling block subcommands walk. Grows a
/// driven-port field per subcommand family as they land.
pub struct ConfigStack {
    service: Box<dyn ConfigAccess>,
    levels: Box<dyn ReadConfigLevel>,
    content: Box<dyn ReadContent>,
}

impl ConfigStack {
    #[must_use]
    pub fn new(
        service: Box<dyn ConfigAccess>,
        levels: Box<dyn ReadConfigLevel>,
        content: Box<dyn ReadContent>,
    ) -> Self {
        Self {
            service,
            levels,
            content,
        }
    }

    #[must_use]
    pub fn config(&self) -> &dyn ConfigAccess {
        self.service.as_ref()
    }

    #[must_use]
    pub fn levels(&self) -> &dyn ReadConfigLevel {
        self.levels.as_ref()
    }

    #[must_use]
    pub fn content(&self) -> &dyn ReadContent {
        self.content.as_ref()
    }
}
