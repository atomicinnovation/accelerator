//! The live-tenant contract harness.
//!
//! Named exactly `contract.rs`, so `cli/.config/nextest.toml`'s
//! `not binary(=contract)` keeps it out of every default run. The lane that
//! selects it — `test:integration:tracker-contract` — is out of the
//! `test:integration` roll-up and out of `default`, because its dependencies
//! are external: a real team, credentials no CI job holds, and network egress.
//!
//! It is therefore only ever invoked deliberately, and an unconfigured run
//! **fails** naming the variables it wants. The port's invariants are enforced
//! offline by `contract_offline.rs` regardless.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use linear_client::filter::FixedStates;
use linear_client::transport::Transport;
use linear_client::{LinearClient, UploadTransport};
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker_support::{
    ClockJitter, CommandPolicy, CredentialContext, SystemEnvironment,
    SystemSleeper, TransportConfig,
};
use tracker_test_support::contract::{run_all, ContractSubject};

/// A live run resolves its token from the environment, where no provenance
/// question arises; wiring the repository's own VCS is Phase 7's concern.
struct NothingTracked;

impl tracker_support::Provenance for NothingTracked {
    fn is_tracked(&self, _path: &Path) -> bool {
        false
    }
}

/// A live run configures itself through `ACCELERATOR_LINEAR_*`, so the config
/// surface it needs is the empty one: the ladder's first two rungs are
/// environment sources.
struct EnvironmentOnlyConfig;

impl config::ConfigAccess for EnvironmentOnlyConfig {
    fn get(
        &self,
        key: &config::Key,
        _level: Option<config::Level>,
    ) -> Result<config::Resolved, config::ConfigError> {
        if key.to_string() != "linear.team_id" {
            return Ok(config::Resolved::Absent);
        }
        Ok(std::env::var("ACCELERATOR_LINEAR_TEAM_ID").map_or(
            config::Resolved::Absent,
            |value| {
                config::Resolved::Found(config::Value::Scalar(
                    config::Scalar::String(value),
                ))
            },
        ))
    }

    fn set(
        &self,
        _key: &config::Key,
        _value: &str,
        _level: config::Level,
    ) -> Result<(), config::ConfigError> {
        unreachable!("a contract run never writes config")
    }
}

struct LiveClient {
    client: LinearClient,
    unaccountable: ExternalId,
    unreadable: ExternalId,
}

impl ContractSubject for LiveClient {
    fn tracker(&self) -> &dyn RemoteTracker {
        &self.client
    }

    /// An identifier outside the configured team: in scope for no search this
    /// client can run, so a complete retrieval still cannot account for it.
    /// The 250-item bulk truncation resolves the same way, which is what makes
    /// this nominable against a live tenant.
    fn unaccountable_id(&self) -> ExternalId {
        self.unaccountable.clone()
    }

    fn unreadable_id(&self) -> ExternalId {
        self.unreadable.clone()
    }
}

fn live_client() -> LiveClient {
    let environment = SystemEnvironment;
    let provenance = NothingTracked;
    let config = EnvironmentOnlyConfig;
    let root = std::env::current_dir().expect("a working directory");
    let integrations: PathBuf =
        std::env::var("ACCELERATOR_LINEAR_CONTRACT_STATE").map_or_else(
            |_| root.join(".accelerator/state/integrations"),
            PathBuf::from,
        );
    let context = CredentialContext {
        environment: &environment,
        config: &config,
        provenance: &provenance,
        personal_config: root.join(".accelerator/config.local.md"),
        insecure_marker: root.join(".claude/insecure-local-ok"),
        command: CommandPolicy::rooted_at(root.clone()),
    };
    let credentials =
        linear_client::resolve_credentials(&context, &integrations).expect(
            "a live contract run needs a team: set ACCELERATOR_LINEAR_TOKEN \
             and ACCELERATOR_LINEAR_TEAM_ID (or a catalogue.json under \
             ACCELERATOR_LINEAR_CONTRACT_STATE)",
        );
    let team_key = std::env::var("ACCELERATOR_LINEAR_CONTRACT_TEAM_KEY")
        .expect(
            "a live contract run needs ACCELERATOR_LINEAR_CONTRACT_TEAM_KEY — \
         the identifier prefix that decides whether an id was in scope",
        );
    let transport = Transport::to_linear(
        credentials,
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )
    .expect("the transport builds");

    LiveClient {
        client: LinearClient::new(
            transport,
            UploadTransport::production().expect("the upload transport builds"),
            Some(team_key),
            Box::new(FixedStates::default()),
        ),
        unaccountable: ExternalId::new(
            std::env::var("ACCELERATOR_LINEAR_CONTRACT_UNACCOUNTABLE")
                .unwrap_or_else(|_| "ZZZZ-999999".to_owned()),
        ),
        unreadable: ExternalId::new(
            std::env::var("ACCELERATOR_LINEAR_CONTRACT_UNREADABLE")
                .unwrap_or_else(|_| "ZZZZ-999998".to_owned()),
        ),
    }
}

#[test]
fn the_conformance_set_passes_against_a_live_team() {
    let subject = live_client();
    let ids = vec![subject.unaccountable_id()];

    let executed = run_all(&subject, &ids)
        .expect("the harness gate must be open for a live run");

    assert!(executed > 0, "the conformance set asserted nothing");
}

#[test]
fn an_unaccounted_id_is_indeterminate_against_a_live_team() {
    let subject = live_client();
    tracker_test_support::contract::unaccounted_id_is_indeterminate_not_absent(
        &subject,
    )
    .expect("the harness gate must be open for a live run");
}

#[test]
fn a_failing_read_is_retryable_against_a_live_team() {
    let subject = live_client();
    tracker_test_support::contract::a_failing_read_is_retryable(&subject)
        .expect("the harness gate must be open for a live run");
}

/// Emit the reduced evidence record for a live run, so the committed
/// `tests/evidence/contract-run.txt` a verifier reads is produced by the
/// harness rather than transcribed by hand. A no-op unless
/// `ACCELERATOR_TRACKER_CONTRACT_EVIDENCE` names an output path, because
/// evidence is generated deliberately, not on every contract run.
#[test]
fn writes_reduced_evidence_when_a_path_is_configured() {
    let Some(path) = std::env::var_os("ACCELERATOR_TRACKER_CONTRACT_EVIDENCE")
    else {
        return;
    };
    let subject = live_client();
    let ids = vec![subject.unaccountable_id()];
    let records =
        tracker_test_support::contract::timed_conformance(&subject, &ids)
            .expect("the harness gate must be open for a live run");
    let date = std::env::var("ACCELERATOR_TRACKER_CONTRACT_DATE").ok();
    let rendered = tracker_test_support::evidence::render(
        "linear",
        date.as_deref(),
        &records,
    );
    std::fs::write(&path, rendered)
        .unwrap_or_else(|error| panic!("write evidence to {path:?}: {error}"));
}
