//! Pins every recipe-1 digest to a committed value, so a change to the
//! canonical body form, a tracker's round trip, the synced field set or the
//! Markdown parser cannot move a persisted baseline without a recipe bump.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use corpus::body::equivalent;
use corpus::body::BodyParser as _;
use corpus_adapters::CommonMarkBody;
use jira_client::JiraBodyDialect;
use serde_json::Value;
use tracker::BodyDialect;
use tracker::PlainDialect;
use work_adapters::sync::baseline::DigestRecipe;
use work_adapters::sync::body_comparison::BodyComparison;
use work_adapters::sync::digest;
use work_adapters::sync::synced_fields::local_fields;
use work_adapters::sync::synced_fields::remote_fields;

type TestError = Box<dyn std::error::Error>;

struct Golden {
    pairs: Vec<BodyPair>,
    fields: Vec<FieldsPair>,
}

struct BodyPair {
    name: String,
    tracker: String,
    local: String,
    remote: String,
    relation: Relation,
    local_body_hash: String,
    remote_body_hash: String,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Relation {
    Cosmetic,
    Meaning,
}

struct FieldsPair {
    name: String,
    local: String,
    remote: String,
    local_fields_hash: String,
    remote_fields_hash: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct RecipeRecord {
    recipe: u64,
    pulldown_cmark: String,
}

fn text(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} is a string in {value}"))
        .to_owned()
}

fn relation(raw: &str) -> Relation {
    match raw {
        "cosmetic" => Relation::Cosmetic,
        "meaning" => Relation::Meaning,
        other => panic!("unrecognised relation: {other}"),
    }
}

fn entries(value: &Value, key: &str) -> Vec<Value> {
    value[key].as_array().cloned().unwrap_or_default()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-digests")
}

fn golden() -> Result<Golden, TestError> {
    let document: Value = serde_json::from_str(&std::fs::read_to_string(
        fixtures().join("pairs.json"),
    )?)?;
    Ok(Golden {
        pairs: entries(&document, "pairs")
            .iter()
            .map(|pair| BodyPair {
                name: text(pair, "name"),
                tracker: text(pair, "tracker"),
                local: text(pair, "local"),
                remote: text(pair, "remote"),
                relation: relation(&text(pair, "relation")),
                local_body_hash: text(pair, "local_body_hash"),
                remote_body_hash: text(pair, "remote_body_hash"),
            })
            .collect(),
        fields: entries(&document, "fields")
            .iter()
            .map(|pair| FieldsPair {
                name: text(pair, "name"),
                local: text(pair, "local"),
                remote: text(pair, "remote"),
                local_fields_hash: text(pair, "local_fields_hash"),
                remote_fields_hash: text(pair, "remote_fields_hash"),
            })
            .collect(),
    })
}

fn dialect(tracker: &str) -> &'static dyn BodyDialect {
    match tracker {
        "linear" => &PlainDialect,
        "jira" => &JiraBodyDialect,
        other => panic!("no dialect for tracker {other}"),
    }
}

fn comparison(pair: &BodyPair) -> BodyComparison<'static> {
    BodyComparison::new(dialect(&pair.tracker), &CommonMarkBody)
}

fn digests(pair: &BodyPair) -> (String, String) {
    let comparison = comparison(pair);
    (
        comparison.local_body_hash(&pair.local),
        comparison.remote_body_hash(&pair.remote),
    )
}

#[test]
fn every_golden_side_digests_to_its_recorded_hash() -> Result<(), TestError> {
    let golden = golden()?;
    let mut moved = Vec::new();
    for pair in &golden.pairs {
        let (local, remote) = digests(pair);
        if local != pair.local_body_hash {
            moved.push(format!("{}\tlocal_body_hash\t{local}", pair.name));
        }
        if remote != pair.remote_body_hash {
            moved.push(format!("{}\tremote_body_hash\t{remote}", pair.name));
        }
    }
    for pair in &golden.fields {
        let local = digest::fields(&local_fields(&pair.local)?);
        let remote = digest::fields(&remote_fields(&pair.remote));
        if local != pair.local_fields_hash {
            moved.push(format!("{}\tlocal_fields_hash\t{local}", pair.name));
        }
        if remote != pair.remote_fields_hash {
            moved.push(format!("{}\tremote_fields_hash\t{remote}", pair.name));
        }
    }
    assert!(
        moved.is_empty(),
        "digests moved from their golden values; bump the digest recipe \
         before re-recording them:\n{}",
        moved.join("\n")
    );
    Ok(())
}

#[test]
fn cosmetic_pairs_hash_equal_and_meaning_pairs_hash_differently(
) -> Result<(), TestError> {
    for pair in golden()?.pairs {
        let (local, remote) = digests(&pair);
        let relation = if local == remote {
            Relation::Cosmetic
        } else {
            Relation::Meaning
        };
        assert_eq!(relation, pair.relation, "{}", pair.name);
    }
    for pair in golden()?.fields {
        assert_eq!(
            digest::fields(&local_fields(&pair.local)?),
            digest::fields(&remote_fields(&pair.remote)),
            "{}",
            pair.name
        );
    }
    Ok(())
}

#[test]
fn equivalence_agrees_with_hash_equality_for_every_pair(
) -> Result<(), TestError> {
    for pair in golden()?.pairs {
        let comparison = comparison(&pair);
        let Ok(rendered) = comparison.local_body(&pair.remote) else {
            continue;
        };
        let pushed = dialect(&pair.tracker).as_pushed(&pair.local);
        let (local, remote) = digests(&pair);
        assert_eq!(
            equivalent(
                &CommonMarkBody.parse(&pushed),
                &CommonMarkBody.parse(&rendered)
            ),
            local == remote,
            "{}",
            pair.name
        );
    }
    Ok(())
}

fn recipe_history() -> Result<Vec<RecipeRecord>, TestError> {
    let document: Value = serde_json::from_str(&std::fs::read_to_string(
        fixtures().join("recipe.json"),
    )?)?;
    Ok(entries(&document, "history")
        .iter()
        .map(|entry| {
            record(
                entry["recipe"].as_u64().expect("recipe is a number"),
                &text(entry, "pulldown_cmark"),
            )
        })
        .collect())
}

fn locked_version(package: &str) -> Result<String, TestError> {
    let lock = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock"),
    )?;
    lock.split("[[package]]")
        .find_map(|entry| {
            entry.contains(&format!("name = \"{package}\"")).then(|| {
                entry
                    .lines()
                    .find_map(|line| line.strip_prefix("version = "))
                    .map(|value| value.trim_matches('"').to_owned())
            })?
        })
        .ok_or_else(|| format!("{package} is not in cli/Cargo.lock").into())
}

/// The history as of its latest entry; `recipe.json` may only extend it.
fn recorded_history() -> Vec<RecipeRecord> {
    vec![record(1, "0.13.4"), record(1, "0.13.5")]
}

fn record(recipe: u64, version: &str) -> RecipeRecord {
    RecipeRecord {
        recipe,
        pulldown_cmark: version.to_owned(),
    }
}

/// Whether `next` keeps `previous`'s recipe, re-verified under another
/// parser version, or bumps it by one.
const fn follows(previous: &RecipeRecord, next: &RecipeRecord) -> bool {
    next.recipe == previous.recipe || next.recipe == previous.recipe + 1
}

/// Whether `history` extends `recorded` one recipe at a time and ends at
/// `current`, last verified under the `locked` parser version.
fn recipe_guard(
    recorded: &[RecipeRecord],
    history: &[RecipeRecord],
    current: u64,
    locked: &str,
) -> Result<(), String> {
    if !history.starts_with(recorded) {
        return Err("an earlier recipe's history entry was rewritten".into());
    }
    let first = history.first().ok_or("the history is empty")?;
    if first.recipe != 1
        || history.windows(2).any(|pair| !follows(&pair[0], &pair[1]))
    {
        return Err("recipes must run 1, 2, 3, … with no gap".into());
    }
    let latest = history.last().ok_or("the history is empty")?;
    if latest.recipe != current {
        return Err(format!(
            "the latest recorded recipe is {}, but the code is at {current}",
            latest.recipe
        ));
    }
    if latest.pulldown_cmark != locked {
        return Err(format!(
            "recipe {current} was last verified under pulldown-cmark {}, but \
             {locked} is locked; record {locked} against recipe {current} if \
             every golden digest holds under it, or bump the recipe",
            latest.pulldown_cmark
        ));
    }
    Ok(())
}

#[test]
fn the_current_recipe_records_the_locked_parser_version(
) -> Result<(), TestError> {
    recipe_guard(
        &recorded_history(),
        &recipe_history()?,
        DigestRecipe::CURRENT,
        &locked_version("pulldown-cmark")?,
    )?;
    Ok(())
}

#[test]
fn the_recipe_guard() {
    let one = record(1, "0.13.4");
    let cases: [(&str, Vec<RecipeRecord>, u64, &str, bool); 8] = [
        ("matching", vec![one.clone()], 1, "0.13.4", true),
        ("mismatched version", vec![one.clone()], 1, "0.13.5", false),
        (
            "recipe re-verified under a new parser",
            vec![one.clone(), record(1, "0.13.5")],
            1,
            "0.13.5",
            true,
        ),
        (
            "recipe going back",
            vec![one.clone(), record(2, "0.13.5"), record(1, "0.13.6")],
            1,
            "0.13.6",
            false,
        ),
        (
            "skipped recipe",
            vec![one.clone(), record(3, "0.13.5")],
            3,
            "0.13.5",
            false,
        ),
        (
            "bumped recipe with a new entry",
            vec![one.clone(), record(2, "0.13.5")],
            2,
            "0.13.5",
            true,
        ),
        (
            "rewritten earlier entry",
            vec![record(1, "0.13.5"), record(2, "0.13.5")],
            2,
            "0.13.5",
            false,
        ),
        (
            "bumped recipe without an entry",
            vec![one],
            2,
            "0.13.4",
            false,
        ),
    ];
    for (name, history, current, locked, passes) in cases {
        assert_eq!(
            recipe_guard(&[record(1, "0.13.4")], &history, current, locked)
                .is_ok(),
            passes,
            "{name}"
        );
    }
}
