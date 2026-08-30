//! The emitter/validator symmetry guard: every value the canonical emitter
//! renders must be accepted by the canonical-quoting validator, so a future
//! divergence between the two (which scan raw text vs render a value tree)
//! fails here rather than as a red corpus self-check.
#![allow(clippy::expect_used)]

use document::{Mapping, Scalar, Yaml};

fn base_work_item(note: Scalar) -> Yaml {
    let mut mapping = Mapping::new();
    let string = |value: &str| Yaml::Scalar(Scalar::String(value.to_owned()));
    mapping.push("type".to_owned(), string("work-item"));
    mapping.push("id".to_owned(), string("0001"));
    mapping.push("title".to_owned(), string("t"));
    mapping.push("date".to_owned(), string("2026-01-01T00:00:00Z"));
    mapping.push("author".to_owned(), string("a"));
    mapping.push("tags".to_owned(), Yaml::Sequence(Vec::new()));
    mapping.push("last_updated".to_owned(), string("2026-01-01T00:00:00Z"));
    mapping.push("last_updated_by".to_owned(), string("a"));
    mapping.push("schema_version".to_owned(), Yaml::Scalar(Scalar::Int(1)));
    mapping.push("status".to_owned(), string("draft"));
    mapping.push("kind".to_owned(), string("task"));
    mapping.push("priority".to_owned(), string("normal"));
    mapping.push("note".to_owned(), Yaml::Scalar(note.clone()));
    mapping.push(
        "extras".to_owned(),
        Yaml::Sequence(vec![Yaml::Scalar(note)]),
    );
    Yaml::Mapping(mapping)
}

fn unquoted_string_count(frontmatter: &str) -> usize {
    corpus::frontmatter_validation::validate_file(frontmatter)
        .iter()
        .filter(|violation| violation.code() == "UNQUOTED-STRING")
        .count()
}

#[test]
fn every_emitter_output_is_accepted_by_the_validator() {
    let adversarial = [
        Scalar::String("plain".to_owned()),
        Scalar::String("a \"quoted\" word".to_owned()),
        Scalar::String("has [brackets]".to_owned()),
        Scalar::String("0042".to_owned()),
        Scalar::String("work-item:0001 # note".to_owned()),
        Scalar::String("a, b".to_owned()),
        Scalar::Int(7),
        Scalar::Bool(true),
        Scalar::Float(1.5),
    ];
    for note in adversarial {
        let rendered = document::render(None, &base_work_item(note.clone()))
            .expect("render");
        let frontmatter =
            document::split(&rendered).expect("split").frontmatter;
        assert_eq!(
            unquoted_string_count(&frontmatter),
            0,
            "emitter output was flagged unquoted for {note:?}:\n{rendered}"
        );
    }
}

#[test]
fn a_bare_mutation_of_an_emitter_output_is_rejected() {
    let rendered = document::render(
        None,
        &base_work_item(Scalar::String("plain".to_owned())),
    )
    .expect("render");
    let frontmatter = document::split(&rendered).expect("split").frontmatter;
    let bared = frontmatter.replace("note: \"plain\"", "note: plain");
    assert!(
        unquoted_string_count(&bared) >= 1,
        "a bare mutation must be flagged:\n{bared}"
    );
}
