//! The pre-search entity resolver a broadened discovery drives.
//!
//! A base-only pull resolves its scope through the adapter's pure
//! `resolve_scope` and issues no extra request. A broadened pull —
//! `additional_*` or whole-workspace — resolves its entities here instead:
//! against the credential's *live* visible set, so a configured entity the
//! credential cannot see aborts the pull rather than silently searching a
//! narrower scope. Single-sourcing the membership rule keeps it from drifting
//! between the two adapters.

use tracker::EntityScope;
use tracker::RemoteTracker;
use tracker::ScopeError;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::VisibleEntity;

/// Why resolving a broadened scope's entities failed.
///
/// The split is the whole point: a named entity absent from the visible set is
/// a configuration fault the operator must fix (a fail-loud, zero-write abort),
/// whereas a failed enumeration is a passing transport condition the caller
/// degrades around exactly as it does a failed discovery search. Collapsing the
/// two would either retry forever on a genuine misconfiguration or abort a whole
/// sync on a network blip.
#[derive(Debug)]
pub enum EntityResolution {
    /// A named base or additional entity is not among the credential's visible
    /// entities.
    Unconfigured(ScopeError),
    /// The visible-entity enumeration itself failed transiently.
    Transient(TrackerError),
}

/// Whether a scope broadens beyond the base entity, and so resolves through the
/// live enumeration here rather than the adapter's pure `resolve_scope`.
#[must_use]
pub const fn is_broadened(scope: &SearchScope) -> bool {
    match &scope.entities {
        EntityScope::Keyed { additional, .. } => !additional.is_empty(),
        EntityScope::WholeWorkspace => true,
    }
}

/// Resolves a broadened scope's entities against the credential's live visible
/// set, mapping each config key to the identifier a search lowers.
///
/// # Errors
///
/// [`EntityResolution::Unconfigured`] when a named base or additional entity is
/// not visible; [`EntityResolution::Transient`] when the enumeration fails.
pub fn resolve_entities(
    tracker: &dyn RemoteTracker,
    scope: &SearchScope,
) -> Result<SearchScope, EntityResolution> {
    let EntityScope::Keyed { base, additional } = &scope.entities else {
        // Whole-workspace resolution lands in a later phase; a base-only scope
        // never reaches here.
        return Ok(scope.clone());
    };
    let visible = tracker
        .enumerate_visible_entities()
        .map_err(EntityResolution::Transient)?;
    let base = base
        .as_deref()
        .map(|key| identifier_for(key, &visible))
        .transpose()?;
    let additional = additional
        .iter()
        .map(|key| identifier_for(key, &visible))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SearchScope {
        entities: EntityScope::Keyed { base, additional },
        filters: scope.filters.clone(),
    })
}

/// The search identifier a visible entity with the given `key` carries, or an
/// unconfigured abort when no visible entity has that key.
fn identifier_for(
    key: &str,
    visible: &[VisibleEntity],
) -> Result<String, EntityResolution> {
    visible
        .iter()
        .find(|entity| entity.key == key)
        .map(|entity| entity.identifier.clone())
        .ok_or_else(|| {
            EntityResolution::Unconfigured(ScopeError {
                detail: format!(
                    "the configured pull scope names {key:?}, which is not \
                     among the entities this credential can see; check the \
                     spelling or the credential's access"
                ),
            })
        })
}
