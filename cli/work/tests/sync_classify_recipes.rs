//! Runs `work::sync::classify` against the recipe-1 and pull-residue table
//! in `tests/fixtures/work-item-sync-classify-recipes.json`, which postdates
//! the frozen bash oracle and so lives beside it rather than in it.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::cell::Cell;
use std::path::Path;

use serde_json::Value;
use tracker::ExternalId;
use tracker::RemoteTimestamp;
use work::normalise::filter_frontmatter_keys;
use work::normalise::trim_lines;
use work::sync::classify;
use work::sync::BaselineEntry;
use work::sync::BaselineFingerprints;
use work::sync::Classification;
use work::sync::ItemComparison;
use work::sync::ItemDigests;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::PullResidue;
use work::sync::RemoteFingerprints;
use work::sync::RemotePresence;
use work::sync::SplitFingerprints;
use work::sync::Subject;
use work::sync::SyncState;
use work::sync::SyncedReconciliation;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_CASES: usize = 15;

fn fixture() -> Result<Value, TestError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-sync-classify-recipes.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn local_hash(content: &str) -> String {
    let (frontmatter, body) = content
        .split_once("---\n")
        .and_then(|(_, rest)| rest.split_once("---\n"))
        .expect("fixture local_content has a frontmatter fence");
    let filtered = filter_frontmatter_keys(frontmatter);
    let trimmed_body = trim_lines(body);
    format!(
        "{}\n{}\n",
        filtered.trim_end_matches('\n'),
        trimmed_body.trim_end_matches('\n')
    )
}

fn remote_hash(body: &str) -> String {
    trim_lines(body)
}

const STALE: &str = "sync-fixture-stale-baseline-value";

fn resolve_hash<'a>(
    symbol: &str,
    from_content: &'a str,
    storage: &'a mut Option<String>,
) -> Option<&'a str> {
    match symbol {
        "absent" => None,
        "stale" => Some(STALE),
        "from-content" => {
            *storage = Some(from_content.to_owned());
            storage.as_deref()
        }
        other => panic!("unrecognised symbolic hash value: {other}"),
    }
}

fn timestamp(value: &Value, placeholders: (&str, &str)) -> RemoteTimestamp {
    let (reported, ticked) = placeholders;
    match value["kind"].as_str().expect("kind is present") {
        "reported" => {
            let raw = value["value"].as_str().expect("value is present");
            let resolved = match raw {
                "$R" => reported,
                "$T" => ticked,
                other => other,
            };
            RemoteTimestamp::Reported(resolved.to_owned())
        }
        "not_reported" => RemoteTimestamp::NotReported,
        "not_read" => RemoteTimestamp::NotRead,
        other => panic!("unrecognised remote_updated kind: {other}"),
    }
}

fn local_fingerprints() -> LocalFingerprints {
    LocalFingerprints {
        frontmatter: "frontmatter".to_owned(),
        fields: "fields".to_owned(),
        body: "body".to_owned(),
    }
}

fn remote_fingerprints() -> RemoteFingerprints {
    RemoteFingerprints {
        body: "remote-body".to_owned(),
        fields: "remote-fields".to_owned(),
    }
}

fn recorded_local(symbol: &str) -> Option<LocalFingerprints> {
    let stale = || STALE.to_owned();
    match symbol {
        "absent" => None,
        "from-content" => Some(local_fingerprints()),
        "stale" => Some(LocalFingerprints {
            frontmatter: stale(),
            fields: stale(),
            body: stale(),
        }),
        "frontmatter-edited" => Some(LocalFingerprints {
            frontmatter: stale(),
            ..local_fingerprints()
        }),
        other => panic!("unrecognised symbolic local fingerprints: {other}"),
    }
}

fn recorded_remote(symbol: &str) -> Option<RemoteFingerprints> {
    match symbol {
        "absent" => None,
        "from-content" => Some(remote_fingerprints()),
        "stale" => Some(RemoteFingerprints {
            body: STALE.to_owned(),
            fields: STALE.to_owned(),
        }),
        other => panic!("unrecognised symbolic remote fingerprints: {other}"),
    }
}

struct FakeDigests {
    mtime: Option<u64>,
    local: String,
    remote_body: Option<String>,
    mtime_called: Cell<bool>,
    local_called: Cell<bool>,
    remote_body_called: Cell<bool>,
}

impl FakeDigests {
    const fn new(
        mtime: Option<u64>,
        local: String,
        remote: Option<String>,
    ) -> Self {
        Self {
            mtime,
            local,
            remote_body: remote,
            mtime_called: Cell::new(false),
            local_called: Cell::new(false),
            remote_body_called: Cell::new(false),
        }
    }
}

impl ItemDigests for FakeDigests {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        self.mtime_called.set(true);
        Ok(self.mtime)
    }

    fn legacy_local(&self) -> Result<String, kernel::Error> {
        self.local_called.set(true);
        Ok(self.local.clone())
    }

    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
        self.remote_body_called.set(true);
        Ok(self.remote_body.clone())
    }

    fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
        self.local_called.set(true);
        Ok(local_fingerprints())
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        self.remote_body_called.set(true);
        Ok(self.remote_body.as_ref().map(|_| remote_fingerprints()))
    }
}

struct FakeComparison {
    residue: Option<PullResidue>,
    called: Cell<bool>,
}

impl ItemComparison for FakeComparison {
    fn pull_residue(&self) -> Option<PullResidue> {
        self.called.set(true);
        self.residue
    }
}

fn residue(case: &Value) -> Option<PullResidue> {
    match case["residue"].as_str() {
        None | Some("title-line-only") => None,
        Some("tracker-wire-document") => Some(PullResidue::TrackerWireDocument),
        Some("verbatim-remote") => Some(PullResidue::VerbatimRemote),
        Some(other) => panic!("unrecognised residue: {other}"),
    }
}

fn expected(raw: &str, name: &str) -> Classification {
    match raw {
        "synced" => Classification::Synced(SyncedReconciliation::ByContent),
        "synced/record" => {
            Classification::Synced(SyncedReconciliation::RecordOnly)
        }
        other => match SyncState::from_keyword(other) {
            Some(SyncState::Unsynced) => Classification::Unsynced,
            Some(SyncState::LocallyModified) => Classification::LocallyModified,
            Some(SyncState::RemotelyModified) => {
                Classification::RemotelyModified
            }
            Some(SyncState::Conflict) => Classification::Conflict,
            Some(SyncState::RemoteAbsent) => Classification::RemoteAbsent,
            Some(SyncState::Indeterminate) => Classification::Indeterminate,
            Some(SyncState::Synced) | None => {
                panic!("unrecognised expected state in {name}")
            }
        },
    }
}

const fn no_residue() -> FakeComparison {
    FakeComparison {
        residue: None,
        called: Cell::new(false),
    }
}

#[test]
fn every_recipe_case_classifies_as_expected() -> Result<(), TestError> {
    let fixture = fixture()?;
    let base_timestamp = fixture["baseline_timestamp"]
        .as_u64()
        .expect("baseline_timestamp is present");
    let local_content = fixture["local_content"]
        .as_str()
        .expect("local_content is present");
    let remote_body_content = fixture["remote_body"]
        .as_str()
        .expect("remote_body is present");
    let reported = fixture["remote_updated_reported"]
        .as_str()
        .expect("remote_updated_reported is present");
    let ticked = fixture["remote_updated_ticked"]
        .as_str()
        .expect("remote_updated_ticked is present");

    let local_from_content = local_hash(local_content);
    let remote_from_content = remote_hash(remote_body_content);

    let cases = fixture["cases"].as_array().expect("cases is an array");
    let mut ran = 0;

    for case in cases {
        ran += 1;

        let name = case["name"].as_str().expect("name is present");
        let external_id_raw = case["external_id"]
            .as_str()
            .expect("external_id is present");
        let external_id = if external_id_raw.is_empty() {
            None
        } else {
            Some(ExternalId::new(external_id_raw.to_owned()))
        };

        let presence =
            match case["presence"].as_str().expect("presence is present") {
                "present" => RemotePresence::Present,
                "absent" => RemotePresence::Absent,
                "indeterminate" => RemotePresence::Indeterminate,
                other => panic!("unrecognised presence: {other}"),
            };

        let expect =
            expected(case["expect"].as_str().expect("expect is present"), name);

        if !matches!(presence, RemotePresence::Present) {
            let subject = Subject {
                external_id: external_id.as_ref(),
                presence,
                remote_updated: &RemoteTimestamp::NotReported,
                baseline_timestamp: 0,
            };
            let baseline = BaselineEntry {
                remote_updated_at: &RemoteTimestamp::NotReported,
                fingerprints: BaselineFingerprints::Legacy(
                    LegacyHashes::default(),
                ),
            };
            let digests = FakeDigests::new(None, String::new(), None);
            let actual =
                classify(&subject, &baseline, &digests, &no_residue())?;
            assert_eq!(actual, expect, "case: {name}");
            continue;
        }

        let remote_updated =
            timestamp(&case["remote_updated"], (reported, ticked));
        let mtime = if case["mtime_absent"].as_bool().unwrap_or(false) {
            None
        } else {
            let offset = case["mtime_offset"]
                .as_i64()
                .unwrap_or_else(|| panic!("mtime_offset missing in {name}"));
            Some(
                base_timestamp
                    .checked_add_signed(offset)
                    .expect("offset stays within u64 range"),
            )
        };
        let with_remote_body =
            case["with_remote_body"].as_bool().unwrap_or(false);

        let fingerprints = match case["recipe"].as_str() {
            Some("legacy") => {
                let mut local_storage = None;
                let mut remote_storage = None;
                let local_symbol = case["baseline"]["local_hash"]
                    .as_str()
                    .expect("baseline.local_hash is present");
                let remote_symbol = case["baseline"]["remote_hash"]
                    .as_str()
                    .expect("baseline.remote_hash is present");
                BaselineFingerprints::Legacy(LegacyHashes {
                    local: resolve_hash(
                        local_symbol,
                        &local_from_content,
                        &mut local_storage,
                    )
                    .map(str::to_owned),
                    remote: resolve_hash(
                        remote_symbol,
                        &remote_from_content,
                        &mut remote_storage,
                    )
                    .map(str::to_owned),
                })
            }
            Some("1") => BaselineFingerprints::Split(SplitFingerprints {
                local: recorded_local(
                    case["baseline"]["local"]
                        .as_str()
                        .expect("baseline.local is present"),
                ),
                remote: recorded_remote(
                    case["baseline"]["remote"]
                        .as_str()
                        .expect("baseline.remote is present"),
                ),
            }),
            other => panic!("unrecognised recipe in {name}: {other:?}"),
        };

        let baseline_updated = timestamp(
            &case["baseline"]["remote_updated_at"],
            (reported, ticked),
        );

        let subject = Subject {
            external_id: external_id.as_ref(),
            presence,
            remote_updated: &remote_updated,
            baseline_timestamp: base_timestamp,
        };
        let baseline = BaselineEntry {
            remote_updated_at: &baseline_updated,
            fingerprints,
        };
        let digests = FakeDigests::new(
            mtime,
            local_from_content.clone(),
            with_remote_body.then(|| remote_from_content.clone()),
        );
        let comparison = FakeComparison {
            residue: residue(case),
            called: Cell::new(false),
        };

        let actual = classify(&subject, &baseline, &digests, &comparison)?;
        assert_eq!(actual, expect, "case: {name}");

        if let Some(expect_mtime) =
            case["rust_only"]["expect_mtime_called"].as_bool()
        {
            assert_eq!(
                digests.mtime_called.get(),
                expect_mtime,
                "case: {name} — mtime() call observability"
            );
        }
        if let Some(expect_local) =
            case["rust_only"]["expect_local_digest_called"].as_bool()
        {
            assert_eq!(
                digests.local_called.get(),
                expect_local,
                "case: {name} — local() call observability"
            );
        }
        if let Some(expect_remote) =
            case["rust_only"]["expect_remote_body_called"].as_bool()
        {
            assert_eq!(
                digests.remote_body_called.get(),
                expect_remote,
                "case: {name} — remote_body() call observability"
            );
        }
        if let Some(expect_residue) =
            case["rust_only"]["expect_pull_residue_called"].as_bool()
        {
            assert_eq!(
                comparison.called.get(),
                expect_residue,
                "case: {name} — pull_residue() call observability"
            );
        }
    }

    assert_eq!(ran, EXPECTED_CASES, "the table's row count has drifted");
    Ok(())
}
