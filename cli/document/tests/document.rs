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
