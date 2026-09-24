//! The sync engine driven by the real provider clients, not a trait double.
//!
//! `sync_run.rs` proves the engine over `RecordingTracker`; this proves the
//! same engine reaches a live classification through `JiraClient` and
//! `LinearClient` pointed at a `MockServer` — the seam where a `TrackerError`
//! class becomes a sync classification and a `FetchOutcome` becomes present,
//! absent or indeterminate. The clients are built through their public
//! constructors with a loopback base, the admission `from_config` refuses.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use http_test_support::MockServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use jira_client::jql::FixedResolver;
use jira_client::transport::Transport as JiraTransport;
use jira_client::Credentials as JiraCredentials;
use jira_client::JiraClient;
use linear_client::cache::LinearCache;
use linear_client::cache::SystemFilesystem;
use linear_client::catalogue::Catalogue;
use linear_client::catalogue::CatalogueUpdate;
use linear_client::catalogue::LiveCatalogueData;
use linear_client::catalogue::TeamEntries;
use linear_client::healing::CatalogueBackfill;
use linear_client::healing::NoBackfill;
use linear_client::resolution::FixedNames;
use linear_client::resolution::ResolverSet;
use linear_client::transport::Transport as LinearTransport;
use linear_client::Credentials as LinearCredentials;
use linear_client::{LinearClient, UploadTransport};
use reqwest::Url;
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker_support::Jitter;
use tracker_support::Secret;
use tracker_support::Sleeper;
use tracker_support::TokenSource;
use tracker_support::TransportConfig;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work_adapters::sync::baseline_store::BaselineStore;
use work_adapters::sync::digest;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::run::run;
use work_adapters::sync::run::ItemSelection;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

type TestError = Box<dyn std::error::Error>;

const STAMP: &str = "2026-06-01T00:00:00.000+0000";
const LINEAR_STAMP: &str = "2026-06-01T00:00:00.000Z";
const BASELINE_PATH: &str = "/baseline/last-sync.json";

/// The catalogue key → UUID pairing the Linear client resolves a scope through.
/// Local to this crate because `linear-client`'s test constants are not visible
/// here.
const LINEAR_TEAM_KEY: &str = "ENG";
const LINEAR_TEAM_ID: &str = "5c9f2a1b-0000-4000-8000-000000000001";

#[derive(Default)]
struct Spy {
    files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
    writes: RefCell<Vec<PathBuf>>,
}

impl Spy {
    fn seed(&self, path: &str, content: &str) {
        self.files
            .borrow_mut()
            .insert(PathBuf::from(path), content.as_bytes().to_vec());
    }
}

impl FileReader for Spy {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        Ok(self
            .files
            .borrow()
            .get(path)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned()))
    }
}

impl AtomicWrite for Spy {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        self.writes.borrow_mut().push(path.to_path_buf());
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }
}

struct AlwaysClean;

impl WorkingCopyStatus for AlwaysClean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

/// The default scope disables discovery and every item here carries an
/// external id, so neither create path — and so neither author method — runs.
struct UnusedAuthor;

impl work_adapters::sync::create::LocalAuthor for UnusedAuthor {
    fn author_from_remote(
        &self,
        _issue: &work_adapters::sync::create::DiscoveredIssue,
    ) -> Result<work_adapters::sync::create::AuthoredLocal, kernel::Error> {
        panic!("the real-client scenarios never author from a discovery")
    }

    fn link_external_id(
        &self,
        _path: &Path,
        _external_id: &ExternalId,
    ) -> Result<(), kernel::Error> {
        panic!("the real-client scenarios never link an external id")
    }
}

struct FixedClock(u64);

impl work::sync::RunClock for FixedClock {
    fn run_start_epoch(&self) -> Result<u64, kernel::Error> {
        Ok(self.0)
    }
}

struct NoSleep;

impl Sleeper for NoSleep {
    fn sleep(&mut self, _duration: Duration) {}
}

struct NoJitter;

impl Jitter for NoJitter {
    fn offset(&mut self, _spread: u64) -> i64 {
        0
    }
}

fn jira_client(base: &str, config: TransportConfig) -> JiraClient {
    let transport = JiraTransport::new(
        JiraCredentials {
            base: Url::parse(base).expect("a base URL"),
            email: "toby@example.com".to_owned(),
            token: Secret::new("secret".to_owned()),
            source: TokenSource::Env,
        },
        config,
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the jira transport builds");
    JiraClient::new(
        transport,
        "ENG".to_owned(),
        Box::new(FixedResolver::new()),
        Box::new(FixedResolver::new()),
    )
}

fn linear_client(base: &str, config: TransportConfig) -> LinearClient {
    linear_client_over(
        base,
        config,
        ResolverSet::new(
            Box::new(FixedNames::default()),
            TeamEntries::keyed(&[(LINEAR_TEAM_KEY, LINEAR_TEAM_ID)]),
        ),
        Arc::new(NoBackfill),
    )
}

fn linear_client_over(
    base: &str,
    config: TransportConfig,
    resolvers: ResolverSet,
    backfill: Arc<dyn CatalogueBackfill>,
) -> LinearClient {
    let transport = LinearTransport::new(
        Url::parse(&format!("{base}/graphql")).expect("an endpoint"),
        LinearCredentials {
            token: Secret::new("lin_api_secret".to_owned()),
            team_id: LINEAR_TEAM_ID.to_owned(),
            source: TokenSource::Env,
        },
        config,
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the linear transport builds");
    LinearClient::new(
        transport,
        UploadTransport::production().expect("the upload transport builds"),
        Some(LINEAR_TEAM_KEY.to_owned()),
        resolvers,
        backfill,
    )
}

fn baseline_document(entry: &str) -> String {
    format!("{{\"timestamp\":0,\"items\":{{{entry}}}}}\n")
}

fn entry(id: &str, stamp: &str, remote_hash: &str, local_hash: &str) -> String {
    format!(
        "\"{id}\":{{\"remote_updated_at\":\"{stamp}\",\
         \"remote_hash\":\"{remote_hash}\",\"local_hash\":\"{local_hash}\"}}"
    )
}

fn item_file(
    dir: &Path,
    id: &str,
    external: &str,
) -> Result<PathBuf, TestError> {
    let path = dir.join(format!("{id}.md"));
    std::fs::write(
        &path,
        format!("---\nexternal_id: \"{external}\"\n---\n\nLocal body\n"),
    )?;
    Ok(path)
}

fn execute(
    tracker: &dyn RemoteTracker,
    spy: &Spy,
    items: &[LocalItem],
    direction: SyncDirection,
    scope: tracker::SearchScope,
    mode: RunMode,
) -> Result<RunReport, work_adapters::sync::run::RunError> {
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let author = UnusedAuthor;
    let ports = SyncPorts {
        tracker,
        status: &status,
        writer: spy,
        clock: &clock,
        author: &author,
    };
    let mut store = BaselineStore::new(PathBuf::from(BASELINE_PATH), spy, spy);
    let resolutions: BTreeMap<String, Resolution> = BTreeMap::new();
    let integrations_root = std::env::temp_dir();
    let request = SyncRequest {
        corpus: items,
        selection: ItemSelection::All,
        direction,
        strategy: RetrievalStrategy::Bulk,
        resolutions: &resolutions,
        max_pulls: tracker::Ceiling::Bounded(25),
        max_pushes: tracker::Ceiling::Bounded(25),
        mode,
        integrations_root: &integrations_root,
        integration: "jira",
        scope,
    };
    run(&ports, &mut store, &request)
}

/// A push-only run: discovery is skipped, so the pull/push classification is
/// exercised without a search against the mock.
fn push_only_report(
    tracker: &dyn RemoteTracker,
    spy: &Spy,
    items: &[LocalItem],
    mode: RunMode,
) -> RunReport {
    execute(
        tracker,
        spy,
        items,
        SyncDirection::PushOnly,
        tracker::SearchScope::default(),
        mode,
    )
    .unwrap_or_else(|_| panic!("the run must not refuse"))
}

#[test]
fn jira_classifies_a_locally_modified_item_through_the_real_client(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = item_file(dir.path(), "0001", "ENG-1")?;
    let item = LocalItem {
        id: "0001".to_owned(),
        path,
        external_id: Some(ExternalId::new("ENG-1".to_owned())),
    };

    let server = MockServer::start();
    server.route(
        RequestKey::post("/rest/api/3/search/jql"),
        Route::Json {
            status: 200,
            body: format!(
                "{{\"issues\":[{{\"key\":\"ENG-1\",\
                 \"fields\":{{\"updated\":\"{STAMP}\"}}}}]}}"
            ),
        },
    );
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-1"),
        Route::Json {
            status: 200,
            body: format!(
                "{{\"key\":\"ENG-1\",\"fields\":{{\"updated\":\"{STAMP}\",\
                 \"summary\":\"Title\",\"description\":{{\"type\":\"doc\",\
                 \"content\":[{{\"type\":\"paragraph\",\"content\":\
                 [{{\"type\":\"text\",\"text\":\"Remote body\"}}]}}]}}}}}}"
            ),
        },
    );
    let client = jira_client(&server.base_url(), TransportConfig::default());

    // The known-payload remote hash comes from the real client's own
    // projection, so a classification of LocallyModified proves the engine
    // compared against exactly that hash.
    let remote_hash = digest::remote_body(
        &client.show(&ExternalId::new("ENG-1".to_owned()))?.body,
    );
    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0001", STAMP, &remote_hash, "stale")),
    );

    let report = push_only_report(&client, &spy, &[item], RunMode::Preview);
    assert_eq!(report.reported.len(), 1);
    assert_eq!(
        report.reported[0].planned.state,
        SyncState::LocallyModified,
        "remote unchanged (hash matches) + local changed classifies \
         locally-modified"
    );
    assert_eq!(report.reported[0].planned.action, work::sync::Action::Push);
    Ok(())
}

#[test]
fn jira_aborts_a_capped_keyed_read_and_deletes_nothing() -> Result<(), TestError>
{
    let dir = tempfile::tempdir()?;
    let path = item_file(dir.path(), "0002", "ENG-2")?;
    let before = std::fs::read_to_string(&path)?;
    let item = LocalItem {
        id: "0002".to_owned(),
        path: path.clone(),
        external_id: Some(ExternalId::new("ENG-2".to_owned())),
    };

    let server = MockServer::start();
    // Every page carries a cursor, so the one-page cap is hit: a fail-loud
    // truncation the run aborts on rather than degrades around.
    server.route(
        RequestKey::post("/rest/api/3/search/jql"),
        Route::Json {
            status: 200,
            body: "{\"issues\":[],\"nextPageToken\":\"more\"}".to_owned(),
        },
    );
    let client = jira_client(
        &server.base_url(),
        TransportConfig {
            discovery_max_pages: tracker::Ceiling::Bounded(1),
            keyed_read_max_pages: tracker::Ceiling::Bounded(1),
            ..TransportConfig::default()
        },
    );

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0002", STAMP, "stale", "stale")),
    );

    // Even push-only performs the shared keyed reconcile read, so a cap-hit
    // aborts the whole run — `--push-only` is no bypass.
    let result = execute(
        &client,
        &spy,
        &[item],
        SyncDirection::PushOnly,
        tracker::SearchScope::default(),
        RunMode::Apply,
    );
    assert!(
        matches!(
            result,
            Err(work_adapters::sync::run::RunError::KeyedReadCapped)
        ),
        "a capped keyed read aborts the whole run"
    );
    assert_eq!(
        std::fs::read_to_string(&path)?,
        before,
        "the abort must not write or delete the item"
    );
    Ok(())
}

#[test]
fn linear_classifies_a_locally_modified_item_through_the_real_client(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = item_file(dir.path(), "0001", "ENG-1")?;
    let item = LocalItem {
        id: "0001".to_owned(),
        path,
        external_id: Some(ExternalId::new("ENG-1".to_owned())),
    };

    let show_body = format!(
        "{{\"data\":{{\"issue\":{{\"id\":\"i\",\"identifier\":\"ENG-1\",\
         \"title\":\"Title\",\"updatedAt\":\"{LINEAR_STAMP}\",\
         \"description\":\"Remote body\"}}}}}}"
    );
    let search_body = format!(
        "{{\"data\":{{\"issues\":{{\"nodes\":[{{\"id\":\"i\",\
         \"identifier\":\"ENG-1\",\"title\":\"Title\",\
         \"updatedAt\":\"{LINEAR_STAMP}\"}}],\"pageInfo\":\
         {{\"hasNextPage\":false,\"endCursor\":null}}}}}}}}"
    );

    // Probe the projected body on a show-only mock, then run against a fresh
    // sequence — Linear posts every operation to the one /graphql key.
    let probe = MockServer::start();
    probe.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: show_body.clone(),
        },
    );
    let remote_hash = digest::remote_body(
        &linear_client(&probe.base_url(), TransportConfig::default())
            .show(&ExternalId::new("ENG-1".to_owned()))?
            .body,
    );

    let server = MockServer::start();
    server.route(
        RequestKey::post("/graphql"),
        Route::Sequence(vec![
            Route::Json {
                status: 200,
                body: search_body,
            },
            Route::Json {
                status: 200,
                body: show_body,
            },
        ]),
    );
    let client = linear_client(&server.base_url(), TransportConfig::default());

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0001", LINEAR_STAMP, &remote_hash, "stale")),
    );

    let report = push_only_report(&client, &spy, &[item], RunMode::Preview);
    assert_eq!(report.reported.len(), 1);
    assert_eq!(report.reported[0].planned.state, SyncState::LocallyModified);
    assert_eq!(report.reported[0].planned.action, work::sync::Action::Push);
    Ok(())
}

#[test]
fn linear_aborts_a_capped_keyed_read_and_deletes_nothing(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path = item_file(dir.path(), "0002", "ENG-2")?;
    let before = std::fs::read_to_string(&path)?;
    let item = LocalItem {
        id: "0002".to_owned(),
        path: path.clone(),
        external_id: Some(ExternalId::new("ENG-2".to_owned())),
    };

    let server = MockServer::start();
    // Every page reports a next page, so the one-page cap is hit: a fail-loud
    // truncation the run aborts on rather than degrades around.
    server.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[],\"pageInfo\":\
                   {\"hasNextPage\":true,\"endCursor\":\"more\"}}}}"
                .to_owned(),
        },
    );
    let client = linear_client(
        &server.base_url(),
        TransportConfig {
            discovery_max_pages: tracker::Ceiling::Bounded(1),
            keyed_read_max_pages: tracker::Ceiling::Bounded(1),
            ..TransportConfig::default()
        },
    );

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0002", LINEAR_STAMP, "stale", "stale")),
    );

    // Even push-only performs the shared keyed reconcile read, so a cap-hit
    // aborts the whole run — `--push-only` is no bypass.
    let result = execute(
        &client,
        &spy,
        &[item],
        SyncDirection::PushOnly,
        tracker::SearchScope::default(),
        RunMode::Apply,
    );
    assert!(
        matches!(
            result,
            Err(work_adapters::sync::run::RunError::KeyedReadCapped)
        ),
        "a capped keyed read aborts the whole run"
    );
    assert_eq!(
        std::fs::read_to_string(&path)?,
        before,
        "the abort must not write or delete the item"
    );
    Ok(())
}

const MOVED: &str = "2026-07-01T00:00:00.000+0000";

fn conflict_item_file(
    dir: &Path,
    id: &str,
    external: &str,
    body: &str,
) -> Result<PathBuf, TestError> {
    let path = dir.join(format!("{id}.md"));
    std::fs::write(
        &path,
        format!("---\nexternal_id: \"{external}\"\n---\n\n{body}\n"),
    )?;
    Ok(path)
}

fn jira_show_route(key: &str, remote_text: &str) -> String {
    format!(
        "{{\"key\":\"{key}\",\"fields\":{{\"updated\":\"{MOVED}\",\
         \"summary\":\"Title\",\"description\":{{\"type\":\"doc\",\
         \"content\":[{{\"type\":\"paragraph\",\"content\":\
         [{{\"type\":\"text\",\"text\":\"{remote_text}\"}}]}}]}}}}}}"
    )
}

#[test]
fn jira_builds_a_dossier_per_conflict_with_values_bound_to_each_side(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let path_one =
        conflict_item_file(dir.path(), "0001", "ENG-1", "Local body one")?;
    let path_two =
        conflict_item_file(dir.path(), "0002", "ENG-2", "Local body two")?;
    let items = vec![
        LocalItem {
            id: "0001".to_owned(),
            path: path_one,
            external_id: Some(ExternalId::new("ENG-1".to_owned())),
        },
        LocalItem {
            id: "0002".to_owned(),
            path: path_two,
            external_id: Some(ExternalId::new("ENG-2".to_owned())),
        },
    ];

    let server = MockServer::start();
    server.route(
        RequestKey::post("/rest/api/3/search/jql"),
        Route::Json {
            status: 200,
            body: format!(
                "{{\"issues\":[{{\"key\":\"ENG-1\",\
                 \"fields\":{{\"updated\":\"{MOVED}\"}}}},{{\"key\":\"ENG-2\",\
                 \"fields\":{{\"updated\":\"{MOVED}\"}}}}]}}"
            ),
        },
    );
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-1"),
        Route::Json {
            status: 200,
            body: jira_show_route("ENG-1", "Remote body one"),
        },
    );
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-2"),
        Route::Json {
            status: 200,
            body: jira_show_route("ENG-2", "Remote body two"),
        },
    );
    let client = jira_client(&server.base_url(), TransportConfig::default());

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &format!(
            "{{\"timestamp\":0,\"items\":{{{},{}}}}}\n",
            entry("0001", STAMP, "stale", "stale"),
            entry("0002", STAMP, "stale", "stale"),
        ),
    );

    let report = execute(
        &client,
        &spy,
        &items,
        SyncDirection::Bidirectional,
        tracker::SearchScope {
            entities: tracker::EntityScope::Keyed {
                base: Some("ENG".to_owned()),
                additional: Vec::new(),
            },
            filters: Vec::new(),
        },
        RunMode::Preview,
    )
    .expect("the run must not refuse");

    assert_eq!(report.dossiers.len(), 2, "one dossier per conflict");
    let ids: std::collections::BTreeSet<&str> =
        report.dossiers.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["0001", "0002"].into_iter().collect());

    let first = report
        .dossiers
        .iter()
        .find(|d| d.id == "0001")
        .expect("a dossier for the first conflict");
    let section = first
        .sections
        .first()
        .expect("the projected remote body differs from the local body");
    assert!(
        section.local.contains("Local body one"),
        "the local side is bound to the local file, got: {:?}",
        section.local
    );
    assert!(
        section.remote.contains("Remote body one"),
        "the remote side is bound to the real client's projection, got: {:?}",
        section.remote
    );
    assert!(
        !section.remote.contains("Local body one"),
        "an operand swap would leak the local body into the remote side"
    );
    Ok(())
}

/// A keyed Linear discovery resolves the team key to the UUID, bounds the
/// search to it, and reports the untracked issue as a planned pull.
/// The load-bearing assertion is the captured body carrying `LINEAR_TEAM_ID` —
/// a `MockServer` routes by method+path and does not evaluate the team filter,
/// so the seeded issue surfaces even with the raw key; the raw key in the body
/// is what fails against the old, unresolved gate.
#[test]
fn linear_discovery_bounds_the_search_to_the_resolved_team_uuid() {
    let search_body = format!(
        "{{\"data\":{{\"issues\":{{\"nodes\":[{{\"id\":\"i2\",\
         \"identifier\":\"ENG-2\",\"title\":\"Untracked\",\
         \"updatedAt\":\"{LINEAR_STAMP}\"}}],\"pageInfo\":\
         {{\"hasNextPage\":false,\"endCursor\":null}}}}}}}}"
    );

    let server = MockServer::start();
    server.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: search_body,
        },
    );
    let client = linear_client(&server.base_url(), TransportConfig::default());

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(""));

    let report = execute(
        &client,
        &spy,
        &[],
        SyncDirection::Bidirectional,
        tracker::SearchScope {
            entities: tracker::EntityScope::Keyed {
                base: Some(LINEAR_TEAM_KEY.to_owned()),
                additional: Vec::new(),
            },
            filters: Vec::new(),
        },
        RunMode::Preview,
    )
    .expect("the keyed discovery run must not refuse");

    let created: Vec<&str> = report
        .reported
        .iter()
        .filter(|item| {
            item.planned.action == work::sync::Action::CreateFromRemote
        })
        .map(|item| item.planned.id.as_str())
        .collect();
    assert_eq!(
        created,
        vec!["ENG-2"],
        "the untracked issue is reported as a create-from-remote pull"
    );

    let bodies = server.bodies(&RequestKey::post("/graphql"));
    assert!(
        !bodies.is_empty(),
        "the discovery search must reach the mock"
    );
    for body in &bodies {
        let text = String::from_utf8_lossy(body);
        assert!(
            text.contains(LINEAR_TEAM_ID),
            "every search body must carry the resolved team UUID, got: {text}"
        );
        assert!(
            !text.contains("\"eq\":\"ENG\""),
            "no search body may carry the raw team key, got: {text}"
        );
    }
}

/// A bidirectional Linear run with no key refuses pre-flight and sends
/// nothing. Linear posts search and every mutation to the one `/graphql` route,
/// so "nothing was sent" is checkable only as zero requests.
#[test]
fn linear_discovery_with_no_key_refuses_before_any_request() {
    let server = MockServer::start();
    server.route(
        RequestKey::post("/graphql"),
        Route::Json {
            status: 200,
            body: "{\"data\":{\"issues\":{\"nodes\":[],\"pageInfo\":\
                   {\"hasNextPage\":false,\"endCursor\":null}}}}"
                .to_owned(),
        },
    );
    let client = linear_client(&server.base_url(), TransportConfig::default());

    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(""));

    let error = execute(
        &client,
        &spy,
        &[],
        SyncDirection::Bidirectional,
        tracker::SearchScope::default(),
        RunMode::Preview,
    )
    .err()
    .expect("an unkeyed discovery run must refuse");

    assert!(
        matches!(
            error,
            work_adapters::sync::run::RunError::DiscoveryUnconfigured { .. }
        ),
        "an unkeyed run refuses as DiscoveryUnconfigured, got: {error:?}"
    );
    assert_eq!(
        server.hits(&RequestKey::post("/graphql")),
        0,
        "a pre-flight config refusal must send nothing"
    );
}

const OPS_TEAM_ID: &str = "ops-uuid";
const SECTION_OPERATIONS: [&str; 6] = [
    "TeamIdentities",
    "TeamStates",
    "TeamLabels",
    "WorkspaceLabels",
    "TeamMembers",
    "TeamProjects",
];

fn catalogue_resolvers(catalogue: &serde_json::Value) -> ResolverSet {
    Catalogue::from_text(&catalogue.to_string()).resolver_set()
}

fn complete_base() -> serde_json::Value {
    serde_json::json!({
        "baseTeam": LINEAR_TEAM_ID,
        "labels": [{ "id": "wl-sec", "name": "Security" }],
        "teams": [{
            "id": LINEAR_TEAM_ID, "key": LINEAR_TEAM_KEY, "name": "Eng",
            "states": [{ "id": "s-eng-ip", "name": "In Progress",
                         "type": "started", "position": 1 }],
            "labels": [{ "id": "l-eng-bug", "name": "Bug" }],
            "members": [{ "id": "u-ann", "name": "Ann Lee",
                          "displayName": "ann", "email": "ann@x.io",
                          "active": true }],
            "projects": [{ "id": "p-alpha", "name": "Alpha" }]
        }]
    })
}

fn legacy_base() -> serde_json::Value {
    serde_json::json!({
        "team": { "id": LINEAR_TEAM_ID, "key": LINEAR_TEAM_KEY, "name": "Eng" },
        "workflowStates": [{ "id": "s-eng-ip", "name": "In Progress",
                             "type": "started", "position": 1 }]
    })
}

fn connection(root: &str, nodes: &serde_json::Value) -> Route {
    Route::Json {
        status: 200,
        body: serde_json::json!({ "data": { root: {
            "nodes": nodes,
            "pageInfo": { "hasNextPage": false, "endCursor": null }
        } } })
        .to_string(),
    }
}

fn serve_issues(server: &MockServer) {
    server.route(
        RequestKey::graphql("issues"),
        connection("issues", &serde_json::json!([])),
    );
}

fn sent_issue_filter(server: &MockServer) -> serde_json::Value {
    let body = server
        .last_body(&RequestKey::graphql("issues"))
        .expect("an issues request");
    let sent: serde_json::Value =
        serde_json::from_slice(&body).expect("JSON body");
    sent["variables"]["filter"].clone()
}

fn section_requests(server: &MockServer) -> usize {
    SECTION_OPERATIONS
        .iter()
        .map(|operation| server.hits(&RequestKey::graphql(operation)))
        .sum()
}

fn base_scope(filters: &[(&str, &str)]) -> tracker::SearchScope {
    tracker::SearchScope {
        entities: tracker::EntityScope::Keyed {
            base: Some(LINEAR_TEAM_KEY.to_owned()),
            additional: Vec::new(),
        },
        filters: filters
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

fn discover(
    client: &LinearClient,
    scope: tracker::SearchScope,
) -> Result<RunReport, work_adapters::sync::run::RunError> {
    let spy = Spy::default();
    spy.seed(BASELINE_PATH, &baseline_document(""));
    execute(
        client,
        &spy,
        &[],
        SyncDirection::Bidirectional,
        scope,
        RunMode::Preview,
    )
}

fn serve_label_fetch(server: &MockServer) {
    server.route(
        RequestKey::graphql("TeamIdentities"),
        connection(
            "teams",
            &serde_json::json!([{ "id": LINEAR_TEAM_ID,
                                  "key": LINEAR_TEAM_KEY, "name": "Eng" }]),
        ),
    );
    server.route(
        RequestKey::graphql("TeamLabels"),
        connection(
            "issueLabels",
            &serde_json::json!([{ "id": "l-eng-bug", "name": "Bug",
                                  "archivedAt": null,
                                  "team": { "id": LINEAR_TEAM_ID } }]),
        ),
    );
    server.route(
        RequestKey::graphql("WorkspaceLabels"),
        connection("issueLabels", &serde_json::json!([])),
    );
}

#[derive(Default)]
struct RecordingBackfill(Mutex<Vec<LiveCatalogueData>>);

impl CatalogueBackfill for RecordingBackfill {
    fn hold(&self, live: LiveCatalogueData) {
        self.0.lock().expect("unpoisoned").push(live);
    }
}

#[test]
fn a_linear_pull_with_an_unknown_label_refuses_before_any_request() {
    let server = MockServer::start();
    let client = linear_client_over(
        &server.base_url(),
        TransportConfig::default(),
        catalogue_resolvers(&complete_base()),
        Arc::new(NoBackfill),
    );

    let error = discover(&client, base_scope(&[("label", "typo")]))
        .err()
        .expect("an unknown label refuses the run");

    let work_adapters::sync::run::RunError::DiscoveryUnconfigured { detail } =
        error
    else {
        panic!("a filter refusal is DiscoveryUnconfigured: {error:?}");
    };
    assert!(detail.contains("E_SEARCH_UNKNOWN_LABEL"), "{detail}");
    assert_eq!(server.hits(&RequestKey::post("/graphql")), 0);
}

#[test]
fn a_linear_pull_sends_seeded_label_assignee_state_and_project_ids() {
    let server = MockServer::start();
    serve_issues(&server);
    let client = linear_client_over(
        &server.base_url(),
        TransportConfig::default(),
        catalogue_resolvers(&complete_base()),
        Arc::new(NoBackfill),
    );

    discover(
        &client,
        base_scope(&[
            ("label", "Bug"),
            ("assignee", "ann@x.io"),
            ("state", "In Progress"),
            ("project", "Alpha"),
        ]),
    )
    .expect("every value resolves");

    let filter = sent_issue_filter(&server);
    assert_eq!(filter["labels"]["id"]["eq"], "l-eng-bug");
    assert_eq!(filter["assignee"]["id"]["eq"], "u-ann");
    assert_eq!(filter["state"]["id"]["eq"], "s-eng-ip");
    assert_eq!(filter["project"]["id"]["eq"], "p-alpha");
    assert!(!filter.to_string().contains("name"), "{filter}");
    assert_eq!(section_requests(&server), 0);
}

#[test]
fn a_whole_workspace_state_pull_sends_every_scoped_teams_ids() {
    let server = MockServer::start();
    serve_issues(&server);
    server.route(
        RequestKey::graphql("TeamEnumeration"),
        connection(
            "teams",
            &serde_json::json!([
                { "id": LINEAR_TEAM_ID, "key": LINEAR_TEAM_KEY, "name": "Eng" },
                { "id": OPS_TEAM_ID, "key": "OPS", "name": "Ops" }
            ]),
        ),
    );
    server.route(
        RequestKey::graphql("TeamIdentities"),
        connection(
            "teams",
            &serde_json::json!([{ "id": OPS_TEAM_ID, "key": "OPS",
                                  "name": "Ops" }]),
        ),
    );
    server.route(
        RequestKey::graphql("TeamStates"),
        connection(
            "workflowStates",
            &serde_json::json!([{ "id": "s-ops-ip", "name": "In Progress",
                                  "type": "started", "position": 1,
                                  "archivedAt": null,
                                  "team": { "id": OPS_TEAM_ID } }]),
        ),
    );
    let client = linear_client_over(
        &server.base_url(),
        TransportConfig::default(),
        catalogue_resolvers(&complete_base()),
        Arc::new(NoBackfill),
    );

    discover(
        &client,
        tracker::SearchScope {
            entities: tracker::EntityScope::WholeWorkspace,
            filters: vec![("state".to_owned(), "In Progress".to_owned())],
        },
    )
    .expect("every team carries the state");

    assert_eq!(server.hits(&RequestKey::graphql("TeamStates")), 1);
    assert_eq!(
        sent_issue_filter(&server)["state"]["id"]["in"],
        serde_json::json!(["s-eng-ip", "s-ops-ip"])
    );
}

#[test]
fn a_linear_pull_against_a_legacy_catalogue_fetches_and_succeeds() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_label_fetch(&server);
    let client = linear_client_over(
        &server.base_url(),
        TransportConfig::default(),
        catalogue_resolvers(&legacy_base()),
        Arc::new(NoBackfill),
    );

    discover(&client, base_scope(&[("label", "Bug")]))
        .expect("the fetched labels resolve the value");

    assert_eq!(
        sent_issue_filter(&server)["labels"]["id"]["eq"],
        "l-eng-bug"
    );
    assert_eq!(server.hits(&RequestKey::graphql("TeamLabels")), 1);
}

#[test]
fn a_later_pull_after_recording_makes_no_section_fetch() -> Result<(), TestError>
{
    let root = tempfile::tempdir()?;
    let state_dir = root.path().join("linear");
    std::fs::create_dir_all(&state_dir)?;
    std::fs::write(
        state_dir.join("catalogue.json"),
        legacy_base().to_string(),
    )?;
    let first = MockServer::start();
    serve_issues(&first);
    serve_label_fetch(&first);
    let backfill = Arc::new(RecordingBackfill::default());
    let client = linear_client_over(
        &first.base_url(),
        TransportConfig::default(),
        Catalogue::load(root.path()).resolver_set(),
        backfill.clone(),
    );
    discover(&client, base_scope(&[("label", "Bug")]))
        .expect("the first pull fetches and resolves");
    let filesystem = SystemFilesystem::new(root.path().to_path_buf());
    let cache = LinearCache::new(&filesystem, state_dir);
    for live in backfill.0.lock().expect("unpoisoned").drain(..) {
        cache.record_team_entries(&CatalogueUpdate {
            base_team: None,
            entries: live.entries,
            workspace_labels: live.workspace_labels,
        })?;
    }

    let second = MockServer::start();
    serve_issues(&second);
    let rebuilt = linear_client_over(
        &second.base_url(),
        TransportConfig::default(),
        Catalogue::load(root.path()).resolver_set(),
        Arc::new(NoBackfill),
    );
    discover(&rebuilt, base_scope(&[("label", "Bug")]))
        .expect("the recorded labels resolve the value");

    assert_eq!(section_requests(&second), 0);
    assert_eq!(
        sent_issue_filter(&second)["labels"]["id"]["eq"],
        "l-eng-bug"
    );
    Ok(())
}
