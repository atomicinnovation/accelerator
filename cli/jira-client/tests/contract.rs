//! The live-tenant contract harness.
//!
//! Named exactly `contract.rs`: `cli/.config/nextest.toml` filters the default
//! profile with `not binary(=contract)`, so this binary runs only under
//! `mise run test:integration:tracker-contract`. A name like
//! `tracker_contract.rs` would silently join the default run and make live API
//! calls during `mise run`.
//!
//! The harness enforces a second, independent gate of its own
//! (`ACCELERATOR_TRACKER_CONTRACT=1`), and errors rather than skips when it is
//! closed — so a dropped variable cannot make this exit 0 having asserted
//! nothing.
//!
//! Credentials are a different condition from the gate, and are treated
//! differently. A bare `mise run` reaches this lane on a machine with no Jira
//! tenant, and CI deliberately holds no provider secrets, so an unconfigured
//! run **skips loudly** rather than failing the whole build. A run that *is*
//! configured and then breaks the contract fails. What proves a live run
//! actually happened is the committed evidence file, not this binary's exit
//! status — and the port's invariants are enforced offline by
//! `contract_offline.rs` in the default profile regardless.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::jql::FixedResolver;
use jira_client::transport::Transport;
use jira_client::JiraClient;
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker_support::{
    ClockJitter, CommandPolicy, CredentialContext, SystemEnvironment,
    SystemSleeper, TransportConfig,
};
use tracker_test_support::contract::{run_all, ContractSubject};

/// Provenance from the repository's own VCS is a Phase-7 wiring concern; a
/// live contract run resolves credentials from the environment, where no
/// provenance question arises.
struct NothingTracked;

impl tracker_support::Provenance for NothingTracked {
    fn is_tracked(&self, _path: &std::path::Path) -> bool {
        false
    }
}

struct LiveClient {
    client: JiraClient,
    unaccountable: ExternalId,
    unreadable: ExternalId,
}

impl ContractSubject for LiveClient {
    fn tracker(&self) -> &dyn RemoteTracker {
        &self.client
    }

    /// An id outside every chunk the harness requests, so a complete retrieval
    /// still cannot account for it.
    fn unaccountable_id(&self) -> ExternalId {
        self.unaccountable.clone()
    }

    fn unreadable_id(&self) -> ExternalId {
        self.unreadable.clone()
    }
}

/// `None` when no tenant is configured, which is a skip rather than a failure.
fn live_client() -> Option<LiveClient> {
    let environment = SystemEnvironment;
    let provenance = NothingTracked;
    let root = std::env::current_dir().expect("a working directory");
    let service = config_service(&root);
    let context = CredentialContext {
        environment: &environment,
        config: service.as_ref(),
        provenance: &provenance,
        personal_config: root.join(".accelerator/config.local.md"),
        insecure_marker: root.join(".claude/insecure-local-ok"),
        command: CommandPolicy::rooted_at(root.clone()),
    };
    let Ok(credentials) = jira_client::resolve_credentials(&context) else {
        return None;
    };
    let Ok(project) = std::env::var("ACCELERATOR_JIRA_CONTRACT_PROJECT") else {
        return None;
    };
    let transport = Transport::new(
        credentials,
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )
    .expect("the transport builds");

    Some(LiveClient {
        client: JiraClient::new(
            transport,
            project,
            Box::new(FixedResolver::new()),
            Box::new(FixedResolver::new()),
        ),
        unaccountable: ExternalId::new(
            std::env::var("ACCELERATOR_JIRA_CONTRACT_UNACCOUNTABLE")
                .unwrap_or_else(|_| "ZZZZ-999999".to_owned()),
        ),
        unreadable: ExternalId::new(
            std::env::var("ACCELERATOR_JIRA_CONTRACT_UNREADABLE")
                .unwrap_or_else(|_| "ZZZZ-999998".to_owned()),
        ),
    })
}

/// Announces a skip on stdout so a credentialed operator can tell it from a
/// pass, and so the evidence file's absence has a visible cause.
fn skip(test: &str) {
    println!(
        "SKIP {test}: no Jira tenant configured. Set ACCELERATOR_JIRA_SITE, \
         ACCELERATOR_JIRA_EMAIL, ACCELERATOR_JIRA_TOKEN and \
         ACCELERATOR_JIRA_CONTRACT_PROJECT to run the live harness."
    );
}

/// The config service a live run reads. Kept behind a function so the harness
/// compiles without a config adapter dependency when one is not needed.
fn config_service(root: &std::path::Path) -> Box<dyn config::ConfigAccess> {
    Box::new(EnvironmentOnlyConfig {
        root: root.to_path_buf(),
    })
}

/// A live run configures itself through `ACCELERATOR_JIRA_*` environment
/// variables, so the config surface a contract run needs is the empty one: the
/// credential ladder's first two rungs are environment sources.
struct EnvironmentOnlyConfig {
    root: std::path::PathBuf,
}

impl config::ConfigAccess for EnvironmentOnlyConfig {
    fn get(
        &self,
        key: &config::Key,
        _level: Option<config::Level>,
    ) -> Result<config::Resolved, config::ConfigError> {
        let name = key.to_string();
        let variable = match name.as_str() {
            "jira.site" => "ACCELERATOR_JIRA_SITE",
            "jira.email" => "ACCELERATOR_JIRA_EMAIL",
            _ => return Ok(config::Resolved::Absent),
        };
        Ok(
            std::env::var(variable).map_or(config::Resolved::Absent, |value| {
                config::Resolved::Found(config::Value::Scalar(
                    config::Scalar::String(value),
                ))
            }),
        )
    }

    fn set(
        &self,
        _key: &config::Key,
        _value: &str,
        _level: config::Level,
    ) -> Result<(), config::ConfigError> {
        unreachable!("a contract run never writes config at {:?}", self.root)
    }
}

#[test]
fn the_conformance_set_passes_against_a_live_tenant() {
    let Some(subject) = live_client() else {
        skip("the_conformance_set_passes_against_a_live_tenant");
        return;
    };
    let ids = vec![subject.unaccountable_id()];

    let executed = run_all(&subject, &ids)
        .expect("the harness gate must be open for a live run");

    assert!(executed > 0, "the conformance set asserted nothing");
}

#[test]
fn an_unaccounted_id_is_indeterminate_against_a_live_tenant() {
    let Some(subject) = live_client() else {
        skip("an_unaccounted_id_is_indeterminate_against_a_live_tenant");
        return;
    };
    tracker_test_support::contract::unaccounted_id_is_indeterminate_not_absent(
        &subject,
    )
    .expect("the harness gate must be open for a live run");
}

#[test]
fn a_failing_read_is_retryable_against_a_live_tenant() {
    let Some(subject) = live_client() else {
        skip("a_failing_read_is_retryable_against_a_live_tenant");
        return;
    };
    tracker_test_support::contract::a_failing_read_is_retryable(&subject)
        .expect("the harness gate must be open for a live run");
}
