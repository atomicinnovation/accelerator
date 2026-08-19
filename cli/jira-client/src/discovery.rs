//! Init discovery.
//!
//! The three GETs that populate the on-disk caches, returned as the cache
//! shapes rather than written. Writing, locking and scaffold upkeep are
//! composition concerns and live in [`crate::cache`].
//!
//! No interactive project-selection prompt exists — a client does not prompt
//! — so `discover_projects` exposes the choices for the skill to render.

use reqwest::Method;
use reqwest::Url;
use serde_json::json;
use serde_json::Value;

use crate::client::JiraClient;
use crate::surface::SurfaceError;

const MYSELF: &str = "/rest/api/3/myself";
const PROJECT: &str = "/rest/api/3/project";
const FIELD: &str = "/rest/api/3/field";

impl JiraClient {
    /// Verifies credentials against `/myself` and returns the `site.json`
    /// shape `{site, accountId}`.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure, a non-2xx status, or a
    /// response carrying no `accountId`.
    pub fn discover_site(&self) -> Result<Value, SurfaceError> {
        let response = self.get_json(MYSELF, "discover site")?;
        let account_id = response
            .get("accountId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| SurfaceError::BadResponse {
                operation: "discover site",
                reason: "the /myself response carried no accountId".to_owned(),
            })?;
        Ok(json!({ "site": self.site(), "accountId": account_id }))
    }

    /// Discovers projects and returns the `projects.json` shape
    /// `{site, projects: [{key, id, name}]}`.
    ///
    /// # Errors
    ///
    /// As [`JiraClient::discover_site`].
    pub fn discover_projects(&self) -> Result<Value, SurfaceError> {
        let response = self.get_json(PROJECT, "discover projects")?;
        let projects = response
            .as_array()
            .ok_or_else(|| SurfaceError::BadResponse {
                operation: "discover projects",
                reason: "the /project response was not an array".to_owned(),
            })?
            .iter()
            .map(project_entry)
            .collect::<Vec<_>>();
        Ok(json!({ "site": self.site(), "projects": projects }))
    }

    /// Discovers fields and returns the `fields.json` shape
    /// `{site, fields: [{id, key, name, slug, schema?}]}`.
    ///
    /// # Errors
    ///
    /// As [`JiraClient::discover_site`].
    pub fn discover_fields(&self) -> Result<Value, SurfaceError> {
        let response = self.get_json(FIELD, "discover fields")?;
        let fields = response
            .as_array()
            .ok_or_else(|| SurfaceError::BadResponse {
                operation: "discover fields",
                reason: "the /field response was not an array".to_owned(),
            })?
            .iter()
            .map(field_entry)
            .collect::<Vec<_>>();
        Ok(json!({ "site": self.site(), "fields": fields }))
    }

    /// The site label the cache records: the Cloud subdomain for an
    /// `*.atlassian.net` base, the bare host otherwise.
    fn site(&self) -> String {
        site_label(self.transport().base())
    }

    fn get_json(
        &self,
        path: &str,
        operation: &'static str,
    ) -> Result<Value, SurfaceError> {
        let received = self.transport().send(&Method::GET, path, &[], None)?;
        if received.status < 200 || received.status >= 300 {
            return Err(SurfaceError::status(
                operation,
                received.status,
                received.body,
            ));
        }
        serde_json::from_str(&received.body).map_err(|error| {
            SurfaceError::BadResponse {
                operation,
                reason: format!("the response was not JSON: {error}"),
            }
        })
    }
}

fn site_label(base: &Url) -> String {
    let host = base.host_str().unwrap_or_default();
    host.strip_suffix(".atlassian.net")
        .unwrap_or(host)
        .to_owned()
}

fn project_entry(project: &Value) -> Value {
    json!({
        "key": project.get("key"),
        "id": project.get("id"),
        "name": project.get("name"),
    })
}

fn field_entry(field: &Value) -> Value {
    let name = field
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut entry = json!({
        "id": field.get("id"),
        "key": field.get("key"),
        "name": field.get("name"),
        "slug": slugify(name),
    });
    if let Some(schema) = schema_projection(field) {
        entry["schema"] = schema;
    }
    entry
}

/// `.schema.custom` and `.schema.type`, kept only when one is present.
fn schema_projection(field: &Value) -> Option<Value> {
    let custom = field.pointer("/schema/custom");
    let kind = field.pointer("/schema/type");
    if custom.is_none() && kind.is_none() {
        return None;
    }
    let mut schema = serde_json::Map::new();
    if let Some(custom) = custom {
        schema.insert("custom".to_owned(), custom.clone());
    }
    if let Some(kind) = kind {
        schema.insert("type".to_owned(), kind.clone());
    }
    Some(Value::Object(schema))
}

/// The slug recipe: ASCII-lowercase, collapse every run of non-`[a-z0-9]` to a
/// single `-`, then trim leading and trailing `-`.
fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut pending_dash = false;
    for character in name.chars() {
        let lowered = character.to_ascii_lowercase();
        if lowered.is_ascii_lowercase() || lowered.is_ascii_digit() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(lowered);
        } else {
            pending_dash = true;
        }
    }
    slug
}
