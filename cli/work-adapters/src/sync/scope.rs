//! The pre-search entity resolver a broadened discovery drives.
//!
//! Every pull first passes its scope through the adapter's pure
//! `resolve_scope`, which validates the filters and, for a base-only pull,
//! resolves its entity with no extra request. A broadened pull —
//! `additional_*` or whole-workspace — then resolves its entities here:
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
/// two would either retry forever on a genuine misconfiguration or abort a
/// whole sync on a network blip.
#[derive(Debug)]
pub enum EntityResolution {
    /// A named base or additional entity is not among the credential's visible
    /// entities.
    Unconfigured(ScopeError),
    /// The visible-entity enumeration itself failed transiently.
    Transient(TrackerError),
}

/// Whether a scope broadens beyond the base entity, and so resolves its
/// entities through the live enumeration here after the adapter's pure
/// `resolve_scope` has validated it.
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
    let visible = tracker
        .enumerate_visible_entities()
        .map_err(EntityResolution::Transient)?;
    let entities = match &scope.entities {
        EntityScope::WholeWorkspace => whole_workspace(&visible)?,
        EntityScope::Keyed { base, additional } => {
            keyed(base.as_deref(), additional, &visible)?
        }
    };
    Ok(SearchScope {
        entities,
        filters: scope.filters.clone(),
    })
}

/// The whole visible workspace as an enumerated identifier list — never an
/// unbounded, constraint-free query. An empty visible set is a refusal, not an
/// empty filter that would flood the workspace.
fn whole_workspace(
    visible: &[VisibleEntity],
) -> Result<EntityScope, EntityResolution> {
    if visible.is_empty() {
        return Err(EntityResolution::Unconfigured(ScopeError {
            detail: "all_projects/all_teams is set but the credential can see \
                     no entities to search; check the credential's access"
                .to_owned(),
        }));
    }
    Ok(EntityScope::Keyed {
        base: None,
        additional: visible
            .iter()
            .map(|entity| entity.identifier.clone())
            .collect(),
    })
}

/// The base and each additional entity mapped to their search identifiers, each
/// confirmed present in the visible set.
fn keyed(
    base: Option<&str>,
    additional: &[String],
    visible: &[VisibleEntity],
) -> Result<EntityScope, EntityResolution> {
    let base = base.map(|key| identifier_for(key, visible)).transpose()?;
    let additional = additional
        .iter()
        .map(|key| identifier_for(key, visible))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EntityScope::Keyed { base, additional })
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
