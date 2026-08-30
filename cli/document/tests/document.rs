use document::{parse, render, split, DocumentError, Mapping, Scalar, Yaml};

fn scalar_at<'a>(node: &'a Yaml, key: &str) -> Option<&'a Scalar> {
    let Yaml::Mapping(mapping) = node else {
        return None;
    };
    match mapping.get(key)? {
        Yaml::Scalar(scalar) => Some(scalar),
        _ => None,
    }
}

#[test]
fn parses_scalar_kinds() -> Result<(), DocumentError> {
    let node = parse("---\nflag: true\ncount: 7\nratio: 1.5\nnothing:\n---\n")?;
    assert_eq!(scalar_at(&node, "flag"), Some(&Scalar::Bool(true)));
    assert_eq!(scalar_at(&node, "count"), Some(&Scalar::Int(7)));
    assert_eq!(scalar_at(&node, "ratio"), Some(&Scalar::Float(1.5)));
    assert_eq!(scalar_at(&node, "nothing"), Some(&Scalar::Null));
    Ok(())
}

#[test]
fn empty_frontmatter_parses_to_an_empty_mapping() -> Result<(), DocumentError> {
    let node = parse("---\n---\n# body\n")?;
    assert_eq!(node, Yaml::Mapping(Mapping::new()));
    Ok(())
}

#[test]
fn an_integer_within_i64_stays_an_int() -> Result<(), DocumentError> {
    let node = parse("---\nn: 9223372036854775807\n---\n")?;
    assert_eq!(scalar_at(&node, "n"), Some(&Scalar::Int(i64::MAX)));
    Ok(())
}

#[test]
fn an_integer_beyond_i64_becomes_a_string() -> Result<(), DocumentError> {
    let node = parse("---\nn: 10000000000000000000\n---\n")?;
    assert_eq!(
        scalar_at(&node, "n"),
        Some(&Scalar::String("10000000000000000000".to_owned()))
    );
    Ok(())
}

#[test]
fn an_integer_beyond_u64_widens_to_float() -> Result<(), DocumentError> {
    let node = parse("---\nn: 99999999999999999999\n---\n")?;
    assert!(matches!(scalar_at(&node, "n"), Some(&Scalar::Float(_))));
    Ok(())
}

#[test]
fn render_round_trips_and_preserves_the_body() -> Result<(), DocumentError> {
    let existing = "---\ncore: old\n---\nbody\n";
    let node = parse("---\ncore: new\n---\nbody\n")?;
    let rendered = render(Some(existing), &node)?;
    let reparsed = parse(&rendered)?;
    assert_eq!(
        scalar_at(&reparsed, "core"),
        Some(&Scalar::String("new".to_owned()))
    );
    assert_eq!(split(&rendered)?.body, "body\n");
    Ok(())
}

#[test]
fn render_preserves_the_body_byte_for_byte() -> Result<(), DocumentError> {
    let cases = [
        "---\na: 1\n---\nbody line\n",
        "---\r\na: 1\r\n---\r\nbody\r\n",
        "---\na: 1\n---\nno trailing newline",
        "---\na: 1\n---\n\nblank first body line\n",
    ];
    for existing in cases {
        let node = parse(existing)?;
        let rendered = render(Some(existing), &node)?;
        assert_eq!(
            split(&rendered)?.body,
            split(existing)?.body,
            "body drift for {existing:?}"
        );
    }
    Ok(())
}

#[test]
fn render_fails_closed_on_fence_valid_but_invalid_yaml() {
    let existing = "---\nkey: : :\n  - broken\n---\nbody\n";
    let node = Yaml::Mapping(Mapping::new());
    assert!(render(Some(existing), &node).is_err());
}

fn string_entry(key: &str, value: &str) -> (String, Yaml) {
    (
        key.to_owned(),
        Yaml::Scalar(Scalar::String(value.to_owned())),
    )
}

fn single(key: &str, value: Yaml) -> Yaml {
    let mut mapping = Mapping::new();
    mapping.push(key.to_owned(), value);
    Yaml::Mapping(mapping)
}

#[test]
fn every_string_scalar_and_element_is_double_quoted(
) -> Result<(), DocumentError> {
    let node = Yaml::Mapping(
        [
            string_entry("parent", "work-item:0171"),
            string_entry("author", "Toby"),
            (
                "relates_to".to_owned(),
                Yaml::Sequence(vec![
                    Yaml::Scalar(Scalar::String("work-item:0194".to_owned())),
                    Yaml::Scalar(Scalar::String("adr:ADR-0034".to_owned())),
                ]),
            ),
        ]
        .into_iter()
        .collect(),
    );
    let rendered = render(None, &node)?;
    assert!(
        rendered.contains("parent: \"work-item:0171\""),
        "{rendered}"
    );
    assert!(rendered.contains("author: \"Toby\""), "{rendered}");
    assert!(
        rendered.contains("relates_to: [\"work-item:0194\", \"adr:ADR-0034\"]"),
        "{rendered}"
    );
    Ok(())
}

#[test]
fn a_long_string_stays_single_line_double_quoted() -> Result<(), DocumentError>
{
    let long = "a very long title that exceeds eighty columns ".repeat(3);
    let node = single("title", Yaml::Scalar(Scalar::String(long.clone())));
    let rendered = render(None, &node)?;
    assert!(
        rendered.contains(&format!("title: \"{long}\"")),
        "{rendered}"
    );
    assert!(!rendered.contains(">-"), "{rendered}");
    assert!(!rendered.contains(">\n"), "{rendered}");
    assert!(!rendered.contains('|'), "{rendered}");
    Ok(())
}

#[test]
fn a_bare_typed_scalar_stays_bare() -> Result<(), DocumentError> {
    let node = Yaml::Mapping(
        [
            ("schema_version".to_owned(), Yaml::Scalar(Scalar::Int(1))),
            ("draft".to_owned(), Yaml::Scalar(Scalar::Bool(true))),
            ("parent".to_owned(), Yaml::Scalar(Scalar::Null)),
        ]
        .into_iter()
        .collect(),
    );
    let rendered = render(None, &node)?;
    assert!(rendered.contains("schema_version: 1"), "{rendered}");
    assert!(rendered.contains("draft: true"), "{rendered}");
    assert!(!rendered.contains("\"1\""), "{rendered}");
    Ok(())
}

#[test]
fn a_float_scalar_is_quoted() -> Result<(), DocumentError> {
    let node = single("ratio", Yaml::Scalar(Scalar::Float(1.0)));
    let rendered = render(None, &node)?;
    assert!(rendered.contains("ratio: \"1\""), "{rendered}");
    Ok(())
}
