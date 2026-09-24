//! Resolves a `RemoteTracker` from the `work.integration` config key.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use config::ConfigAccess;
use jira_client::JiraClient;
use linear_client::discovery::TeamEntryFetch;
use linear_client::healing::CatalogueBackfill;
use linear_client::healing::CatalogueHealing;
use linear_client::healing::NoBackfill;
use linear_client::transport::Url;
use linear_client::LinearClient;
use tracker::RemoteTracker;
use tracker_support::CommandPolicy;
use tracker_support::CredentialContext;
use tracker_support::Environment;
use tracker_support::Provenance;
use tracker_support::SystemEnvironment;
use tracker_support::TransportConfig;
use tracker_support::INSECURE_MARKER_RELATIVE;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs_adapters::library::InProcessProbe;

pub enum SelectionError {
    Unset,
    Unrecognised {
        name: String,
    },
    NotAvailable {
        name: String,
    },
    Unconfigured {
        name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl SelectionError {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Unset => "work.integration is not set. Recognised \
                trackers: linear, jira, trello, github-issues. Configure \
                one with /accelerator:configure."
                .to_owned(),
            Self::Unrecognised { name } => format!(
                "work.integration names an unrecognised tracker: '{name}'. \
                 Recognised trackers: linear, jira, trello, github-issues."
            ),
            Self::NotAvailable { name } => format!(
                "work.integration names '{name}', which is recognised but \
                 has no client wired yet."
            ),
            // Rendered from the source chain, not a stored string, so the
            // caller can still tell a missing site from a failed credential
            // helper from a shared-config refusal.
            Self::Unconfigured { name, source } => format!(
                "work.integration names '{name}', which is wired but not \
                 usable — its configuration or credentials are missing or \
                 refused: {source}"
            ),
        }
    }
}

pub trait TrackerRegistry {
    /// # Errors
    ///
    /// [`SelectionError`] when `name` is empty, unrecognised, names a tracker
    /// with no client built, or names a wired tracker whose configuration or
    /// credentials cannot be resolved.
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError>;
}

/// Answers `Provenance` for the credential trust boundary by reading whether a
/// path is tracked in the repository at `root`, in-process.
///
/// A read failure is treated as untracked, so an unreadable repository
/// loosens the check rather than refusing every credential.
struct VcsProvenance {
    root: PathBuf,
    kind: VcsKind,
}

impl VcsProvenance {
    fn discovered(root: PathBuf) -> Self {
        let kind = InProcessProbe.kind(&root);
        Self { root, kind }
    }
}

impl Provenance for VcsProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        let Ok(relpath) = path.strip_prefix(&self.root) else {
            return false;
        };
        let Some(relpath) = relpath.to_str() else {
            return false;
        };
        match InProcessProbe.is_tracked(&self.root, relpath, self.kind) {
            Ok(tracked) => tracked,
            Err(error) => {
                tracing::warn!(
                    %error,
                    path = %path.display(),
                    "could not determine VCS tracking; treating as untracked"
                );
                false
            }
        }
    }
}

/// Everything the credential ladder reads, assembled from the repository root.
fn credential_context<'a>(
    config: &'a dyn ConfigAccess,
    environment: &'a dyn Environment,
    provenance: &'a dyn Provenance,
    root: &Path,
) -> CredentialContext<'a> {
    CredentialContext {
        environment,
        config,
        provenance,
        personal_config: root.join(".accelerator/config.local.md"),
        insecure_marker: root.join(INSECURE_MARKER_RELATIVE),
        command: CommandPolicy::rooted_at(root.to_path_buf()),
    }
}

/// The production registry. `jira` and `linear` resolve real clients from
/// configuration; `trello` and `github-issues` report not-available; everything
/// else falls to `Unrecognised`.
pub struct ConfiguredTrackers<'a> {
    config: &'a dyn ConfigAccess,
    root: PathBuf,
    healing: Option<Arc<CatalogueHealing>>,
    environment: Option<&'a dyn Environment>,
    linear_endpoint: Option<Url>,
}

impl<'a> ConfiguredTrackers<'a> {
    #[must_use]
    pub const fn new(config: &'a dyn ConfigAccess, root: PathBuf) -> Self {
        Self {
            config,
            root,
            healing: None,
            environment: None,
            linear_endpoint: None,
        }
    }

    /// Every Linear client this registry resolves holds its live team fetches
    /// in `healing`'s buffer.
    #[must_use]
    pub fn with_catalogue_healing(
        mut self,
        healing: Arc<CatalogueHealing>,
    ) -> Self {
        self.healing = Some(healing);
        self
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn with_environment(
        mut self,
        environment: &'a dyn Environment,
    ) -> Self {
        self.environment = Some(environment);
        self
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_linear_endpoint(mut self, endpoint: Url) -> Self {
        self.linear_endpoint = Some(endpoint);
        self
    }

    /// A Linear client for fetching team sections, built from the same
    /// credentials and transport bounds as [`TrackerRegistry::resolve`], whose
    /// fetches are never held.
    ///
    /// # Errors
    ///
    /// [`SelectionError::Unconfigured`] when the Linear configuration or
    /// credential cannot be resolved.
    pub fn linear_team_entry_fetch(
        &self,
    ) -> Result<Box<dyn TeamEntryFetch>, SelectionError> {
        self.linear_client(Arc::new(NoBackfill))
            .map(|client| Box::new(client) as Box<dyn TeamEntryFetch>)
    }

    fn linear_client(
        &self,
        backfill: Arc<dyn CatalogueBackfill>,
    ) -> Result<LinearClient, SelectionError> {
        let name = "linear";
        let integrations_root =
            crate::sync::integrations_dir(self.config, &self.root)
                .map_err(|error| Self::unconfigured(name, error))?;
        let transport_config = self.transport_config(name)?;
        self.with_credential_context(|context| match &self.linear_endpoint {
            Some(endpoint) => LinearClient::from_config_at(
                endpoint.clone(),
                context,
                &integrations_root,
                transport_config,
                backfill,
            ),
            None => LinearClient::from_config(
                context,
                &integrations_root,
                transport_config,
                backfill,
            ),
        })
        .map_err(|error| Self::unconfigured(name, error))
    }

    fn backfill(&self) -> Arc<dyn CatalogueBackfill> {
        self.healing.as_ref().map_or_else(
            || Arc::new(NoBackfill) as Arc<dyn CatalogueBackfill>,
            |healing| healing.backfill(),
        )
    }

    fn with_credential_context<T>(
        &self,
        body: impl FnOnce(&CredentialContext<'_>) -> T,
    ) -> T {
        let system = SystemEnvironment;
        let environment = self.environment.unwrap_or(&system);
        let provenance = VcsProvenance::discovered(self.root.clone());
        body(&credential_context(
            self.config,
            environment,
            &provenance,
            &self.root,
        ))
    }

    fn unconfigured(
        name: &str,
        error: impl std::error::Error + Send + Sync + 'static,
    ) -> SelectionError {
        SelectionError::Unconfigured {
            name: name.to_owned(),
            source: Box::new(error),
        }
    }

    /// The transport bounds for `name`, with the page caps sourced from the
    /// `<name>.pull.max_pages` block. The block is already validated on the
    /// sync path before a client is resolved, so a fault here is defensive.
    fn transport_config(
        &self,
        name: &str,
    ) -> Result<TransportConfig, SelectionError> {
        let ceilings =
            tracker_support::pull::resolve_ceilings(self.config, name)
                .map_err(|detail| {
                    Self::unconfigured(name, PullError(detail))
                })?;
        Ok(TransportConfig {
            discovery_max_pages: ceilings.discovery_pages,
            keyed_read_max_pages: ceilings.keyed_read_pages,
            ..TransportConfig::default()
        })
    }
}

/// A `<tracker>.pull` resolution fault, carrying its already-rendered message
/// so it can route through [`SelectionError::Unconfigured`]'s error source.
#[derive(Debug)]
struct PullError(String);

impl std::fmt::Display for PullError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PullError {}

impl TrackerRegistry for ConfiguredTrackers<'_> {
    fn resolve(
        &self,
        name: &str,
    ) -> Result<Box<dyn RemoteTracker>, SelectionError> {
        match name {
            "" => Err(SelectionError::Unset),
            "jira" => {
                let transport_config = self.transport_config(name)?;
                self.with_credential_context(|context| {
                    JiraClient::from_config(context, transport_config)
                })
                .map(|client| Box::new(client) as Box<dyn RemoteTracker>)
                .map_err(|error| Self::unconfigured(name, error))
            }
            "linear" => self
                .linear_client(self.backfill())
                .map(|client| Box::new(client) as Box<dyn RemoteTracker>),
            "trello" | "github-issues" => Err(SelectionError::NotAvailable {
                name: name.to_owned(),
            }),
            other => Err(SelectionError::Unrecognised {
                name: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::HashMap;

    use ::config::{ConfigError, Key, Level, Resolved, Scalar, Value};
    use http_test_support::{MockServer, RequestKey, Route};
    use linear_client::cache::{LinearCache, SystemFilesystem};
    use linear_client::healing::{FetchUnavailable, SyncedTeams};
    use serde_json::json;
    use tracker::{EntityScope, SearchScope};

    use super::*;

    struct FakeConfig(HashMap<String, String>);

    impl ConfigAccess for FakeConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self
                .0
                .get(&key.to_string())
                .map_or(Resolved::Absent, |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            unreachable!("the registry never writes config")
        }
    }

    fn linear_config() -> FakeConfig {
        FakeConfig(
            [
                ("work.integration", "linear"),
                ("paths.integrations", "integrations"),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
        )
    }

    struct FakeEnvironment(HashMap<String, String>);

    impl Environment for FakeEnvironment {
        fn read(&self, name: &str) -> Option<String> {
            self.0.get(name).cloned()
        }
    }

    fn with_token() -> FakeEnvironment {
        FakeEnvironment(HashMap::from([(
            "ACCELERATOR_LINEAR_TOKEN".to_owned(),
            "lin_api_test".to_owned(),
        )]))
    }

    fn connection(root: &str, nodes: &serde_json::Value) -> Route {
        Route::Json {
            status: 200,
            body: json!({ "data": { root: {
                "nodes": nodes,
                "pageInfo": { "hasNextPage": false, "endCursor": null }
            } } })
            .to_string(),
        }
    }

    fn serve_every_section_of_ops(server: &MockServer) {
        let ops = json!({ "nodes": [{ "id": "t-ops" }],
                          "pageInfo": { "hasNextPage": false } });
        for (operation, root, nodes) in [
            (
                "TeamIdentities",
                "teams",
                json!([{ "id": "t-ops", "key": "OPS", "name": "Ops" }]),
            ),
            (
                "TeamStates",
                "workflowStates",
                json!([{ "id": "s-ops", "name": "Todo", "type": "unstarted",
                         "position": 0, "archivedAt": null,
                         "team": { "id": "t-ops" } }]),
            ),
            (
                "TeamLabels",
                "issueLabels",
                json!([{ "id": "l-ops", "name": "Bug", "archivedAt": null,
                         "team": { "id": "t-ops" } }]),
            ),
            (
                "TeamMembers",
                "users",
                json!([{ "id": "u-ann", "name": "Ann", "displayName": "ann",
                         "email": "ann@x.io", "active": true,
                         "teams": ops }]),
            ),
            (
                "TeamProjects",
                "projects",
                json!([{ "id": "p-alpha", "name": "Alpha",
                         "archivedAt": null, "teams": ops }]),
            ),
        ] {
            server.route(
                RequestKey::graphql(operation),
                connection(root, &nodes),
            );
        }
        server.route(
            RequestKey::graphql("issues"),
            connection("issues", &json!([])),
        );
    }

    fn seeded_root() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("tempdir");
        let linear = root.path().join("integrations/linear");
        std::fs::create_dir_all(&linear).expect("state dir");
        std::fs::write(
            linear.join("catalogue.json"),
            json!({
                "baseTeam": "t-eng",
                "labels": [],
                "teams": [
                    { "id": "t-eng", "key": "ENG", "name": "Eng",
                      "states": [{ "id": "s-eng", "name": "Todo",
                                   "type": "unstarted", "position": 0 }],
                      "labels": [], "members": [], "projects": [] },
                    { "id": "t-ops", "key": "OPS", "name": "Ops" }
                ]
            })
            .to_string(),
        )
        .expect("seed the catalogue");
        root
    }

    #[test]
    fn a_client_from_resolve_holds_into_the_buffer_healing_reads() {
        let server = MockServer::start();
        serve_every_section_of_ops(&server);
        let root = seeded_root();
        let config = linear_config();
        let environment = with_token();
        let healing = Arc::new(CatalogueHealing::new());
        let registry =
            ConfiguredTrackers::new(&config, root.path().to_path_buf())
                .with_catalogue_healing(Arc::clone(&healing))
                .with_environment(&environment)
                .with_linear_endpoint(
                    Url::parse(&format!("{}/graphql", server.base_url()))
                        .expect("an endpoint"),
                );

        let tracker = registry.resolve("linear").ok().expect("a Linear client");
        tracker
            .search(&SearchScope {
                entities: EntityScope::Keyed {
                    base: Some("t-ops".to_owned()),
                    additional: Vec::new(),
                },
                filters: [
                    ("validated:state", "Todo"),
                    ("validated:label", "Bug"),
                    ("validated:assignee", "ann@x.io"),
                    ("validated:project", "Alpha"),
                ]
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            })
            .expect("the search fetches OPS and completes");
        let requests = server.hits(&RequestKey::post("/graphql"));
        let filesystem = SystemFilesystem::new(root.path().to_path_buf());
        let cache = LinearCache::new(
            &filesystem,
            root.path().join("integrations/linear"),
        );
        let no_fetch =
            || -> Result<Box<dyn TeamEntryFetch>, FetchUnavailable> {
                panic!("the held sections complete OPS")
            };

        let outcome = healing.heal(&SyncedTeams::default(), &no_fetch, &cache);

        assert_eq!(outcome.recorded, vec!["OPS"]);
        assert_eq!(server.hits(&RequestKey::post("/graphql")), requests);
    }

    #[test]
    fn without_a_token_the_team_entry_fetch_is_unconfigured() {
        let root = seeded_root();
        let config = linear_config();
        let environment = FakeEnvironment(HashMap::new());
        let registry =
            ConfiguredTrackers::new(&config, root.path().to_path_buf())
                .with_environment(&environment);

        let Err(error) = registry.linear_team_entry_fetch() else {
            panic!("no credential, no fetch");
        };

        assert!(matches!(error, SelectionError::Unconfigured { .. }));
        assert!(error.message().contains("linear"), "{}", error.message());
    }
}
