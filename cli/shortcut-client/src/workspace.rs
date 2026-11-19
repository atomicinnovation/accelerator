//! `GET /api/v4/whoami`: the workspace slug every other path needs, the
//! member, and the token's scopes.

use reqwest::Url;
use serde_json::Value;

use crate::discovery::DiscoveryError;
use crate::paths::Paths;
use crate::paths::WorkspaceSlug;
use crate::transport::Deadline;
use crate::transport::Transport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhoAmI {
    pub slug: WorkspaceSlug,
    pub workspace_name: String,
    pub member_id: String,
    pub member_mention_name: String,
    pub scopes: Vec<String>,
    pub default_state_id: Option<i64>,
}

impl WhoAmI {
    /// Whether the token holds only the read scope.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        !self.scopes.is_empty()
            && self.scopes.iter().all(|scope| scope == "read")
    }
}

/// # Errors
///
/// [`DiscoveryError`] when whoami fails or answers without a usable slug.
pub fn whoami(
    transport: &Transport,
    base: &Url,
    deadline: &Deadline,
) -> Result<WhoAmI, DiscoveryError> {
    let delivery = transport.get(&Paths::whoami(base), deadline);
    let received = delivery.result.map_err(DiscoveryError::Client)?;
    if !received.succeeded() {
        return Err(DiscoveryError::refused(&received, &[]));
    }
    let body = received.json().ok_or_else(|| unusable("non-JSON"))?;
    let entity = body.get("entity").unwrap_or(&body);
    let text = |pointer: &str| entity.pointer(pointer).and_then(Value::as_str);
    let slug_text =
        text("/workspace/slug").ok_or_else(|| unusable("no workspace slug"))?;
    let slug = WorkspaceSlug::parse(slug_text)
        .map_err(|refused| unusable(&refused.to_string()))?;
    let scopes = entity
        .pointer("/authorization/scopes")
        .and_then(Value::as_array)
        .map(|scopes| {
            scopes
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(WhoAmI {
        slug,
        workspace_name: text("/workspace/name").unwrap_or_default().to_owned(),
        member_id: text("/member/id").unwrap_or_default().to_owned(),
        member_mention_name: text("/member/mention_name")
            .unwrap_or_default()
            .to_owned(),
        scopes,
        default_state_id: entity
            .pointer("/default_workflow_state/id")
            .and_then(Value::as_i64),
    })
}

fn unusable(what: &str) -> DiscoveryError {
    DiscoveryError::Unusable {
        what: format!("whoami answered {what}"),
    }
}
