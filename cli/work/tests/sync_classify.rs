//! Runs `work::sync::classify` against the committed table in
//! `tests/fixtures/work-item-sync-classify.json`.
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
use work::sync::ItemDigests;
use work::sync::RemotePresence;
use work::sync::Subject;
use work::sync::SyncState;

type TestError = Box<dyn std::error::Error>;

/// Pinned independently of `applies_to` so narrowing a row's applicability
/// cannot quietly shrink what this suite exercises.
const EXPECTED_RUST_CASES: usize = 17;

fn fixture() -> Result<Value, TestError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-sync-classify.json");
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

struct FakeDigests {
    mtime: Option<u64>,
    local: String,
    remote_body: Option<String>,
    mtime_called: Cell<bool>,
    local_called: Cell<bool>,
    remote_body_called: Cell<bool>,
}

impl ItemDigests for FakeDigests {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        self.mtime_called.set(true);
        Ok(self.mtime)
    }

    fn local(&self) -> Result<String, kernel::Error> {
        self.local_called.set(true);
        Ok(self.local.clone())
    }

    fn remote_body(&self) -> Result<Option<String>, kernel::Error> {
        self.remote_body_called.set(true);
        Ok(self.remote_body.clone())
    }
}

#[test]
fn every_case_classifies_as_expected() -> Result<(), TestError> {
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
        let applies_to_rust = case["applies_to"]
            .as_array()
            .expect("applies_to is an array")
            .iter()
            .any(|v| {
                v.as_str().expect("applies_to entries are strings") == "rust"
            });
        if !applies_to_rust {
            continue;
        }
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

        let expect = SyncState::from_keyword(
            case["expect"].as_str().expect("expect is present"),
        )
        .unwrap_or_else(|| panic!("unrecognised expected state in {name}"));

        if !matches!(presence, RemotePresence::Present) {
            let subject = Subject {
                external_id: external_id.as_ref(),
                presence,
                remote_updated: &RemoteTimestamp::NotReported,
                baseline_timestamp: 0,
            };
            let baseline = BaselineEntry {
                remote_updated_at: &RemoteTimestamp::NotReported,
                remote_hash: None,
                local_hash: None,
            };
            let digests = FakeDigests {
                mtime: None,
                local: String::new(),
                remote_body: None,
                mtime_called: Cell::new(false),
                local_called: Cell::new(false),
                remote_body_called: Cell::new(false),
            };
            let actual = classify(&subject, &baseline, &digests)?;
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

        let mut local_storage = None;
        let mut remote_storage = None;
        let local_symbol = case["baseline"]["local_hash"]
            .as_str()
            .expect("baseline.local_hash is present");
        let remote_symbol = case["baseline"]["remote_hash"]
            .as_str()
            .expect("baseline.remote_hash is present");
        let local_hash_value =
            resolve_hash(local_symbol, &local_from_content, &mut local_storage)
                .map(str::to_owned);
        let remote_hash_value = resolve_hash(
            remote_symbol,
            &remote_from_content,
            &mut remote_storage,
        )
        .map(str::to_owned);

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
            remote_hash: remote_hash_value.as_deref(),
            local_hash: local_hash_value.as_deref(),
        };
        let digests = FakeDigests {
            mtime,
            local: local_from_content.clone(),
            remote_body: if with_remote_body {
                Some(remote_from_content.clone())
            } else {
                None
            },
            mtime_called: Cell::new(false),
            local_called: Cell::new(false),
            remote_body_called: Cell::new(false),
        };

        let actual = classify(&subject, &baseline, &digests)?;
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
    }

    assert_eq!(
        ran, EXPECTED_RUST_CASES,
        "the number of rows this side ran has drifted from the pinned floor"
    );
    Ok(())
}
