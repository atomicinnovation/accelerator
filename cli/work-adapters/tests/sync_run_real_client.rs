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
use linear_client::filter::FixedStates;
use linear_client::transport::Transport as LinearTransport;
use linear_client::Credentials as LinearCredentials;
use linear_client::LinearClient;
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
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;
use work_adapters::sync::run::SyncPorts;
use work_adapters::sync::run::SyncRequest;

type TestError = Box<dyn std::error::Error>;

const STAMP: &str = "2026-06-01T00:00:00.000+0000";
const LINEAR_STAMP: &str = "2026-06-01T00:00:00.000Z";
const BASELINE_PATH: &str = "/baseline/last-sync.json";

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
    let transport = LinearTransport::new(
        Url::parse(&format!("{base}/graphql")).expect("an endpoint"),
        LinearCredentials {
            token: Secret::new("lin_api_secret".to_owned()),
            team_id: "5c9f2a1b-0000-4000-8000-000000000001".to_owned(),
            source: TokenSource::Env,
        },
        config,
        Box::new(NoSleep),
        Box::new(NoJitter),
    )
    .expect("the linear transport builds");
    LinearClient::new(
        transport,
        Some("ENG".to_owned()),
        Box::new(FixedStates::default()),
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
    mode: RunMode,
) -> RunReport {
    let clock = FixedClock(1_700_000_000);
    let status = AlwaysClean;
    let ports = SyncPorts {
        tracker,
        status: &status,
        writer: spy,
        clock: &clock,
    };
    let mut store = BaselineStore::new(PathBuf::from(BASELINE_PATH), spy, spy);
    let resolutions: BTreeMap<String, Resolution> = BTreeMap::new();
    let request = SyncRequest {
        items,
        direction: SyncDirection::Bidirectional,
        strategy: RetrievalStrategy::Bulk,
        resolutions: &resolutions,
        max_pulls: 25,
        max_pushes: 25,
        mode,
    };
    run(&ports, &mut store, &request)
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

    let report = execute(&client, &spy, &[item], RunMode::Preview);
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
fn jira_marks_a_truncated_read_indeterminate_and_deletes_nothing(
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
    // Every page carries a cursor, so the one-page cap is hit and the chunk's
    // keys are unaccounted for.
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
            max_pages: 1,
            ..TransportConfig::default()
        },
    );

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0002", STAMP, "stale", "stale")),
    );

    let report = execute(&client, &spy, &[item], RunMode::Apply);
    assert_eq!(report.reported.len(), 1);
    assert_eq!(
        report.reported[0].planned.state,
        SyncState::Indeterminate,
        "a truncated read must be indeterminate, never remote-absent"
    );
    assert_eq!(
        std::fs::read_to_string(&path)?,
        before,
        "an indeterminate item must not be written or deleted"
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

    let report = execute(&client, &spy, &[item], RunMode::Preview);
    assert_eq!(report.reported.len(), 1);
    assert_eq!(report.reported[0].planned.state, SyncState::LocallyModified);
    assert_eq!(report.reported[0].planned.action, work::sync::Action::Push);
    Ok(())
}

#[test]
fn linear_marks_a_truncated_read_indeterminate_and_deletes_nothing(
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
            max_pages: 1,
            ..TransportConfig::default()
        },
    );

    let spy = Spy::default();
    spy.seed(
        BASELINE_PATH,
        &baseline_document(&entry("0002", LINEAR_STAMP, "stale", "stale")),
    );

    let report = execute(&client, &spy, &[item], RunMode::Apply);
    assert_eq!(report.reported.len(), 1);
    assert_eq!(report.reported[0].planned.state, SyncState::Indeterminate);
    assert_eq!(
        std::fs::read_to_string(&path)?,
        before,
        "an indeterminate item must not be written or deleted"
    );
    Ok(())
}
