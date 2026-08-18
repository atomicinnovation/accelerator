//! JQL composition.
//!
//! Values are **quoted, never concatenated**. Identifiers reach this composer
//! from work-item files, having originally come from a remote tracker; one
//! containing `'`, `)` or ` OR ` would otherwise break out of its clause and
//! change which issues the query returns — turning a targeted fetch into a
//! project dump, or hiding issues that exist.
//!
//! The quoting uses single quotes with any interior single quote doubled, not
//! double quotes with backslash escapes.

use std::collections::BTreeMap;

use crate::error::ClientError;

/// Resolves `@me` to the caller's account id.
///
/// A port from the outset so the cache-backed implementation can land later
/// without a constructor change.
pub trait AccountResolver {
    fn me(&self) -> Option<String>;
}

/// Resolves a field token to its `customfield_NNNNN` id.
pub trait FieldResolver {
    /// `None` means "not in the cache" — the token passes through to Jira.
    fn resolve(&self, token: &str) -> Option<String>;
}

/// A fixed map, used by the search suites and by any caller with no cache.
pub struct FixedResolver {
    pub account: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl FixedResolver {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            account: None,
            fields: BTreeMap::new(),
        }
    }
}

impl Default for FixedResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountResolver for FixedResolver {
    fn me(&self) -> Option<String> {
        self.account.clone()
    }
}

impl FieldResolver for FixedResolver {
    fn resolve(&self, token: &str) -> Option<String> {
        self.fields.get(token).cloned()
    }
}

/// Quotes one JQL value.
///
/// # Errors
///
/// [`ClientError::BadJql`] for an empty value — `IS EMPTY` has its own clause
/// — or for one carrying a control byte, which is not safely quotable.
pub fn quote(value: &str) -> Result<String, ClientError> {
    if value.is_empty() {
        return Err(ClientError::BadJql {
            reason: "E_JQL_EMPTY_VALUE: empty value supplied; use an IS EMPTY \
                     clause instead"
                .to_owned(),
        });
    }
    if let Some(control) = value
        .chars()
        .find(|character| character.is_control() || *character == '\u{7f}')
    {
        return Err(ClientError::BadJql {
            reason: format!(
                "E_JQL_UNSAFE_VALUE: control character {:#04x} in value is \
                 not safely quotable",
                u32::from(control)
            ),
        });
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

/// The `key IN ('A-1', 'A-2')` clause `fetch_all` composes.
///
/// The key clause is the **sole** filter: no `project = <default>` clause is
/// injected, so a cross-project key is never dropped. An out-of-project key
/// coming back unfound from a believed-complete read would be reported absent,
/// and the sync would unlink a live issue.
///
/// # Errors
///
/// [`ClientError::BadJql`] when a key cannot be safely quoted.
pub fn key_clause(keys: &[String]) -> Result<String, ClientError> {
    let mut quoted = Vec::with_capacity(keys.len());
    for key in keys {
        quoted.push(quote(key)?);
    }
    Ok(format!("key IN ({})", quoted.join(", ")))
}

/// A contains-match clause, `<field> ~ "<escaped>"`.
///
/// A different quoting from [`quote`]: Atlassian's double-quoted string rules,
/// where `\` is escaped **before** `"` — reversing the order would double the
/// backslash in front of every quote and produce `\\"` instead of `\"`.
///
/// # Errors
///
/// [`ClientError::BadJql`] when the value carries a control character.
pub fn match_clause(field: &str, value: &str) -> Result<String, ClientError> {
    if value
        .chars()
        .any(|character| character.is_control() || character == '\u{7f}')
    {
        return Err(ClientError::BadJql {
            reason: "E_JQL_BAD_VALUE: control character in match value"
                .to_owned(),
        });
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("{field} ~ \"{escaped}\""))
}

/// One negatable, repeatable flag family: `status`, `labels`, `assignee`,
/// `issuetype`, `component`, `reporter` and `parent`. A leading `~` negates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub field: String,
    pub values: Vec<String>,
}

/// Everything `jql_compose` accepts, in the same shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub project: Option<String>,
    pub all_projects: bool,
    pub families: Vec<Family>,
    pub watching: bool,
    pub text: Vec<String>,
    pub empty: Vec<String>,
    pub not_empty: Vec<String>,
    pub raw: Option<String>,
}

/// Composes the search surface's JQL.
///
/// Clause order: project, `IS EMPTY`, `IS NOT EMPTY`, each value family in its
/// declared order, `watcher`, each text match, then any raw tail.
///
/// # Errors
///
/// [`ClientError::BadJql`] when neither a project nor `all_projects` is given,
/// or when a value cannot be quoted.
pub fn compose(
    search: &Search,
    accounts: &dyn AccountResolver,
    fields: &dyn FieldResolver,
) -> Result<String, ClientError> {
    if search.project.is_none() && !search.all_projects {
        return Err(ClientError::BadJql {
            reason: "E_JQL_NO_PROJECT: specify a project or all_projects"
                .to_owned(),
        });
    }

    let mut clauses = Vec::new();
    if let Some(project) = &search.project {
        clauses.push(format!("project = {}", quote(project)?));
    }
    for field in &search.empty {
        clauses.push(format!(
            "{} IS EMPTY",
            fields.resolve(field).unwrap_or_else(|| field.clone())
        ));
    }
    for field in &search.not_empty {
        clauses.push(format!(
            "{} IS NOT EMPTY",
            fields.resolve(field).unwrap_or_else(|| field.clone())
        ));
    }
    for family in &search.families {
        clauses.extend(family_clauses(family, accounts, fields)?);
    }
    if search.watching {
        clauses.push("watcher = currentUser()".to_owned());
    }
    for value in &search.text {
        clauses.push(match_clause("text", value)?);
    }
    if let Some(raw) = &search.raw {
        clauses.push(raw.clone());
    }

    Ok(clauses.join(" AND "))
}

fn family_clauses(
    family: &Family,
    accounts: &dyn AccountResolver,
    fields: &dyn FieldResolver,
) -> Result<Vec<String>, ClientError> {
    let field = fields
        .resolve(&family.field)
        .unwrap_or_else(|| family.field.clone());
    let mut positives = Vec::new();
    let mut negatives = Vec::new();
    for value in &family.values {
        let (negated, bare) = value
            .strip_prefix('~')
            .map_or((false, value.as_str()), |rest| (true, rest));
        let resolved = resolve_principal(bare, accounts)?;
        if negated {
            negatives.push(quote(&resolved)?);
        } else {
            positives.push(quote(&resolved)?);
        }
    }

    let mut clauses = Vec::new();
    if !positives.is_empty() {
        clauses.push(format!("{field} IN ({})", positives.join(", ")));
    }
    if !negatives.is_empty() {
        clauses.push(format!("{field} NOT IN ({})", negatives.join(", ")));
    }
    Ok(clauses)
}

/// `@me` resolves through the account cache, and an unresolvable `@me` is a
/// refusal rather than a query over the literal token.
fn resolve_principal(
    value: &str,
    accounts: &dyn AccountResolver,
) -> Result<String, ClientError> {
    if value != "@me" {
        return Ok(value.to_owned());
    }
    accounts.me().ok_or_else(|| ClientError::BadJql {
        reason: "E_SEARCH_NO_SITE_CACHE: @me cannot be resolved without a \
                 cached accountId"
            .to_owned(),
    })
}
