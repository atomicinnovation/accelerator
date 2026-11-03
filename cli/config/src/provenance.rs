//! Where a configured path's effective value came from, for a consumer
//! whose trust in the path depends on who could have set it.

use crate::consent::ProvenanceContext;
use crate::consent::Tracking;
use crate::error::ConfigError;
use crate::key::Key;
use crate::service::Source;

/// The source of a path's effective value. A personal value carries the
/// tracking of the file it was read from, since a committed
/// `config.local.md` is no more the user's own than the team config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSource {
    Default,
    Team,
    Personal(Tracking),
}

/// # Errors
///
/// A [`ConfigError`] when a config level cannot be read.
pub fn path_source(
    context: &ProvenanceContext<'_>,
    key: &Key,
) -> Result<PathSource, ConfigError> {
    Ok(match context.config.effective(key, None)?.source() {
        Source::Personal => PathSource::Personal(
            context.tracking.tracking(&context.personal_config),
        ),
        Source::Team => PathSource::Team,
        Source::Catalogue | Source::Unset => PathSource::Default,
    })
}
