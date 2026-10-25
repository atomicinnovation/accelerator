//! The language-neutral access-policy corpus, asserted against the Rust
//! policy. The same fixture drives the JavaScript policy in the Playwright
//! daemon, so a case failing on either side means the two implementations have
//! drifted.

use std::fs;
use std::path::PathBuf;

use design::source_location;
use design::Allowances;
use design::Verdict;

use serde_json::Value;

type TestError = Box<dyn std::error::Error>;

fn corpus() -> Result<Value, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/access-policy-vectors.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn policy_cases(corpus: &Value) -> Result<&Vec<Value>, TestError> {
    corpus["policy"]
        .as_array()
        .ok_or_else(|| "corpus has no `policy` array".into())
}

#[test]
fn every_policy_case_evaluates_as_the_corpus_says() -> Result<(), TestError> {
    let corpus = corpus()?;
    let cases = policy_cases(&corpus)?;
    assert!(!cases.is_empty(), "the policy corpus is empty");

    for case in cases {
        let url = case["url"].as_str().ok_or("a policy case has no `url`")?;
        let allowances = Allowances {
            internal: case["allow_internal"].as_bool().unwrap_or(false),
            insecure_scheme: case["allow_insecure_scheme"]
                .as_bool()
                .unwrap_or(false),
        };
        let expected = case["verdict"]
            .as_str()
            .ok_or("a policy case has no `verdict`")?;

        let verdict =
            source_location::parse(url).map_or("rejected", |location| {
                match design::evaluate(&location, allowances) {
                    Verdict::Accepted => "accepted",
                    Verdict::Rejected(_) => "rejected",
                }
            });
        assert_eq!(verdict, expected, "{url}");
    }
    Ok(())
}
