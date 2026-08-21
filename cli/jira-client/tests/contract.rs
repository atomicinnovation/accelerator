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
//! It needs a live tenant, so `test:integration:tracker-contract` is out of
//! the `test:integration` roll-up and out of `default`: the lane is invoked
//! deliberately, by someone who has configured one. An unconfigured run
//! therefore **fails**, naming the variables it wants, rather than skipping —
//! a harness that passes with nothing configured is the failure mode this
//! whole design exists to avoid.
//!
//! The port's invariants do not depend on this binary: `contract_offline.rs`
//! enforces them against a mock in the default profile. This is the
//! live-tenant assurance beside it, and what proves it ran is the committed
//! evidence file.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::jql::FixedResolver;
use jira_client::transport::Transport;
use jira_client::JiraClient;
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker_support::{
    ClockJitter, CommandPolicy, CredentialContext, SystemEnvironment,
    SystemSleeper, TransportConfig,
};
use tracker_test_support::contract::{run_all, ContractSubject};
use tracker_test_support::seed::{
    guard_target, representative_records, run_seed, ScratchAllowlist,
};

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

    /// A well-formed key with no matching issue, used by the partition test.
    /// A live `/search/jql` returns it as a clean empty result, so the client
    /// files it `absent` — see `can_nominate_indeterminate` below.
    fn unaccountable_id(&self) -> ExternalId {
        self.unaccountable.clone()
    }

    fn unreadable_id(&self) -> ExternalId {
        self.unreadable.clone()
    }

    /// Jira reaches `indeterminate` only through a failed search — a 5xx, a
    /// page-cap or a deadline. A live tenant returns a 2xx empty result for any
    /// benign but unmatched key, which the client correctly files `absent`, so
    /// no id this subject could name is reported indeterminate. The property is
    /// enforced offline instead, where the mock returns 500 for the search
    /// (`contract_offline.rs`). Linear, whose team scope is a structural
    /// indeterminate path, keeps the default.
    fn can_nominate_indeterminate(&self) -> bool {
        false
    }

    /// A live tenant returns a clean, complete result for any benign scope, so
    /// no scope this subject could name yields `complete == false`. The
    /// truncation property is enforced offline, where the mock fails the search
    /// (`contract_offline.rs`).
    fn can_induce_truncation(&self) -> bool {
        false
    }
}

fn live_client() -> LiveClient {
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
    let credentials = jira_client::resolve_credentials(&context).expect(
        "a live contract run needs a tenant: set ACCELERATOR_JIRA_SITE, \
         ACCELERATOR_JIRA_EMAIL and ACCELERATOR_JIRA_TOKEN",
    );
    let project = std::env::var("ACCELERATOR_JIRA_CONTRACT_PROJECT").expect(
        "a live contract run needs ACCELERATOR_JIRA_CONTRACT_PROJECT — the \
         project new issues are created in",
    );
    let transport = Transport::new(
        credentials,
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )
    .expect("the transport builds");

    LiveClient {
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
    }
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
    let subject = live_client();
    let ids = vec![subject.unaccountable_id()];

    let executed = run_all(&subject, &ids)
        .expect("the harness gate must be open for a live run");

    assert!(executed > 0, "the conformance set asserted nothing");
}

// No live equivalent of `unaccounted_id_is_indeterminate`: Jira reaches
// `indeterminate` only through a failed search, which a live tenant will not
// produce for a benign id (`can_nominate_indeterminate` is false). The property
// is enforced offline in `contract_offline.rs`, where the mock fails the
// search.

#[test]
fn a_failing_read_is_retryable_against_a_live_tenant() {
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
        "jira",
        date.as_deref(),
        &records,
    );
    std::fs::write(&path, rendered)
        .unwrap_or_else(|error| panic!("write evidence to {path:?}: {error}"));
}

/// Seeds the scratch Jira project with the representative corpus, idempotently.
///
/// A no-op unless `ACCELERATOR_TRACKER_SEED=1`, so an ordinary contract run
/// never writes issues. When seeding, the contract gate must be open and the
/// target project must be on `ACCELERATOR_TRACKER_SCRATCH_ALLOWLIST` — a
/// non-scratch project is refused before any create. Re-running reuses the
/// already-seeded issues rather than duplicating them.
#[test]
fn seeds_the_scratch_corpus_when_requested() {
    if std::env::var_os("ACCELERATOR_TRACKER_SEED").is_none() {
        return;
    }
    assert_eq!(
        std::env::var("ACCELERATOR_TRACKER_CONTRACT")
            .ok()
            .as_deref(),
        Some("1"),
        "seeding needs the contract gate open: set \
         ACCELERATOR_TRACKER_CONTRACT=1",
    );

    let project = std::env::var("ACCELERATOR_JIRA_CONTRACT_PROJECT")
        .expect("ACCELERATOR_JIRA_CONTRACT_PROJECT names the scratch project");
    let allowlist = ScratchAllowlist::from_list(
        &std::env::var("ACCELERATOR_TRACKER_SCRATCH_ALLOWLIST").expect(
            "ACCELERATOR_TRACKER_SCRATCH_ALLOWLIST lists the scratch \
             projects/teams the seed may write to",
        ),
    );
    guard_target(&allowlist, &project).unwrap_or_else(|refusal| {
        panic!("refusing to seed a non-scratch target: {refusal:?}")
    });

    let subject = live_client();
    let scope = SearchScope {
        project: Some(project),
        all_projects: false,
        filters: Vec::new(),
    };
    let records = representative_records();
    let summary = run_seed(subject.tracker(), &scope, &records)
        .unwrap_or_else(|error| panic!("seed failed: {error:?}"));

    println!(
        "jira seed: {} created, {} reused",
        summary.created.len(),
        summary.reused.len()
    );
    assert_eq!(
        summary.created.len() + summary.reused.len(),
        records.len(),
        "every record is either created or reused",
    );
}
