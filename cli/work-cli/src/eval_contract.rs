//! The `work id check` and `work create --id` output the create-work-item
//! evals quote, held to what the CLI prints, so an eval cannot keep passing
//! against output the CLI no longer gives.

#![allow(clippy::panic)]

use crate::exit_codes;

const EVALS: &str =
    include_str!("../../../skills/work/create-work-item/evals/evals.json");

fn prompts() -> Vec<String> {
    let evals: serde_json::Value =
        serde_json::from_str(EVALS).unwrap_or_else(|error| panic!("{error}"));
    evals["evals"]
        .as_array()
        .unwrap_or_else(|| panic!("evals.json holds an evals list"))
        .iter()
        .filter_map(|evaluation| evaluation["prompt"].as_str())
        .map(str::to_owned)
        .collect()
}

/// Each `exiting <code> with this stderr: … E_ID_*: …` a prompt quotes, as
/// its exit code and the refusal from its code onward.
fn quoted_id_refusals(prompt: &str) -> Vec<(u8, String)> {
    let mut quoted = Vec::new();
    let mut rest = prompt;
    while let Some(start) = rest.find("exiting ") {
        rest = &rest[start + "exiting ".len()..];
        let digits: String =
            rest.chars().take_while(char::is_ascii_digit).collect();
        let window = &rest[..rest.len().min(120)];
        if let (Ok(code), Some(at)) = (digits.parse(), window.find("E_ID_")) {
            let line = window[at..].lines().next().unwrap_or_default();
            quoted.push((code, line.to_owned()));
        }
    }
    quoted
}

/// Each `exact`, `other-key` or `corrected` verdict a prompt quotes.
fn quoted_verdicts(prompt: &str) -> Vec<String> {
    prompt
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|quoted| {
            ["exact\t", "other-key\t", "corrected\t"]
                .iter()
                .any(|verdict| quoted.starts_with(verdict))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_quoted_id_refusal_exits_as_the_cli_exits_it() {
    let quoted: Vec<(u8, String)> = prompts()
        .iter()
        .flat_map(|prompt| quoted_id_refusals(prompt))
        .collect();
    assert!(!quoted.is_empty(), "the evals quote no E_ID_* refusal");
    for (code, line) in quoted {
        assert_eq!(code, exit_codes::of_failure(&line), "{line}");
    }
}

#[test]
fn every_quoted_verdict_has_the_fields_its_kind_prints() {
    let quoted: Vec<String> = prompts()
        .iter()
        .flat_map(|prompt| quoted_verdicts(prompt))
        .collect();
    assert!(!quoted.is_empty(), "the evals quote no verdict");
    for verdict in quoted {
        let fields: Vec<&str> = verdict.split('\t').collect();
        let well_formed = match fields.as_slice() {
            ["exact" | "corrected", id] => !id.is_empty(),
            ["other-key" | "corrected", id, key] => {
                !id.is_empty() && !key.is_empty()
            }
            _ => false,
        };
        assert!(well_formed, "{verdict:?}");
    }
}
