//! The `config` command's core: the composed port bundle the inbound adapter
//! reads through, and the shared failure policy.
//!
//! The bundle is expressed in `config`-crate traits, never the concrete
//! adapter, so no launcher module outside the composition root names
//! `config_adapters`.

pub mod agents;
pub mod context;
pub mod dump;
pub mod get;
pub mod init;
pub mod paths;
pub mod review;
pub mod summary;
pub mod work;

pub mod template;

use config::{
    ConfigAccess, ConfigError, Key, Level, ReadConfigLevel, ReadContent,
    ReadLensCatalogue, ReadTemplate, Resolved, Scaffold, TemplateOverride,
};

/// A resolved scalar subcommand value with the warnings accumulated while
/// resolving it (each emitted on stderr, never on stdout).
pub struct ScalarView {
    pub value: String,
    pub warnings: Vec<String>,
}

/// The `--explain` resolution provenance for a scalar read: which level file
/// supplied the value and which files were consulted. Reports the set/not-set
/// status of both levels — the per-level presence a single winning source
/// cannot reconstruct.
///
/// # Errors
///
/// A [`ConfigError`] when a level being probed cannot be read.
pub(crate) fn explain_lines(
    config: &dyn ConfigAccess,
    key: &Key,
    level: Option<Level>,
    explain: bool,
) -> Result<Vec<String>, ConfigError> {
    if !explain {
        return Ok(Vec::new());
    }
    let mut lines = Vec::new();
    let levels: &[Level] = match level {
        Some(Level::Team) => &[Level::Team],
        Some(Level::Personal) => &[Level::Personal],
        None => &[Level::Team, Level::Personal],
    };
    let mut winner = "default";
    for probe in levels {
        let present =
            matches!(config.get(key, Some(*probe))?, Resolved::Found(_));
        lines.push(format!(
            "{probe} ({}): {}",
            probe.filename(),
            if present { "set" } else { "not set" }
        ));
        if present {
            winner = match probe {
                Level::Team => "team",
                Level::Personal => "personal",
            };
        }
    }
    lines.push(format!("resolved from: {winner}"));
    Ok(lines)
}

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
    lenses: Box<dyn ReadLensCatalogue>,
    templates: Box<dyn ReadTemplate>,
    overrides: Box<dyn TemplateOverride>,
    scaffold: Box<dyn Scaffold>,
}

impl ConfigStack {
    #[must_use]
    pub fn new(
        service: Box<dyn ConfigAccess>,
        levels: Box<dyn ReadConfigLevel>,
        content: Box<dyn ReadContent>,
        lenses: Box<dyn ReadLensCatalogue>,
        templates: Box<dyn ReadTemplate>,
        overrides: Box<dyn TemplateOverride>,
        scaffold: Box<dyn Scaffold>,
    ) -> Self {
        Self {
            service,
            levels,
            content,
            lenses,
            templates,
            overrides,
            scaffold,
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

    #[must_use]
    pub fn lenses(&self) -> &dyn ReadLensCatalogue {
        self.lenses.as_ref()
    }

    #[must_use]
    pub fn templates(&self) -> &dyn ReadTemplate {
        self.templates.as_ref()
    }

    #[must_use]
    pub fn overrides(&self) -> &dyn TemplateOverride {
        self.overrides.as_ref()
    }

    #[must_use]
    pub fn scaffold(&self) -> &dyn Scaffold {
        self.scaffold.as_ref()
    }
}
