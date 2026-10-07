use corpus::frontmatter::FrontmatterError;
use corpus::frontmatter::FrontmatterParser;
use corpus::frontmatter::FrontmatterState;
use corpus::FrontmatterValue;
use corpus::Mapping;
use corpus::Scalar;
use corpus_adapters::YamlFrontmatter;

const ABSENT: &str = "# Notes\n\ncontent\n";
const EMPTY: &str = "---\n---\nbody\n";
const NULL_ROOT: &str = "---\nnull\n---\nbody\n";
const MAPPING: &str = "---\ntitle: Foo\n---\n# Body\n";
const SEQUENCE_ROOT: &str = "---\n- a\n- b\n---\nbody\n";
const SCALAR_ROOT: &str = "---\njust a string\n---\nbody\n";
const UNTERMINATED: &str = "---\ntitle: foo\n";
const INVALID_YAML: &str = "---\nkey: [unclosed\n---\nbody\n";
const TAGGED: &str = "---\nkey: !custom value\n---\nbody\n";

fn string(value: &str) -> FrontmatterValue {
    FrontmatterValue::Scalar(Scalar::String(value.to_owned()))
}

fn title_foo() -> Mapping {
    std::iter::once(("title".to_owned(), string("Foo"))).collect()
}

fn document_error(content: &str) -> FrontmatterError {
    match document::parse(content) {
        Ok(_) => FrontmatterError("expected a document error".to_owned()),
        Err(error) => FrontmatterError(error.to_string()),
    }
}

#[test]
fn classify_judges_each_root_class() {
    let cases = [
        (ABSENT, FrontmatterState::Absent, ABSENT),
        (EMPTY, FrontmatterState::Parsed(Mapping::new()), "body\n"),
        (
            NULL_ROOT,
            FrontmatterState::Parsed(Mapping::new()),
            "body\n",
        ),
        (MAPPING, FrontmatterState::Parsed(title_foo()), "# Body\n"),
        (SEQUENCE_ROOT, FrontmatterState::Malformed, "body\n"),
        (SCALAR_ROOT, FrontmatterState::Malformed, "body\n"),
        (UNTERMINATED, FrontmatterState::Malformed, ""),
        (INVALID_YAML, FrontmatterState::Malformed, "body\n"),
        (TAGGED, FrontmatterState::Malformed, "body\n"),
    ];
    for (content, state, body) in cases {
        let document = YamlFrontmatter.classify(content.as_bytes());
        assert_eq!(document.state, state, "{content:?}");
        assert_eq!(document.body, body, "{content:?}");
    }
}

#[test]
fn classify_reads_invalid_utf8_lossily() {
    let document = YamlFrontmatter.classify(b"---\ntitle: Foo\n---\n\xff\n");
    assert_eq!(document.state, FrontmatterState::Parsed(title_foo()));
    assert_eq!(document.body, "\u{fffd}\n");
}

#[test]
fn parse_value_reads_any_untagged_root_as_a_value() {
    let cases = [
        (ABSENT, FrontmatterValue::Mapping(Mapping::new())),
        (EMPTY, FrontmatterValue::Mapping(Mapping::new())),
        (NULL_ROOT, FrontmatterValue::Scalar(Scalar::Null)),
        (MAPPING, FrontmatterValue::Mapping(title_foo())),
        (
            SEQUENCE_ROOT,
            FrontmatterValue::Sequence(vec![string("a"), string("b")]),
        ),
        (SCALAR_ROOT, string("just a string")),
    ];
    for (content, value) in cases {
        assert_eq!(
            YamlFrontmatter.parse_value(content),
            Ok(value),
            "{content:?}"
        );
    }
}

#[test]
fn parse_value_carries_the_document_error_text_verbatim() {
    assert_eq!(
        YamlFrontmatter.parse_value(UNTERMINATED),
        Err(FrontmatterError(
            "unterminated frontmatter block".to_owned()
        ))
    );
    for content in [INVALID_YAML, TAGGED] {
        assert_eq!(
            YamlFrontmatter.parse_value(content),
            Err(document_error(content)),
            "{content:?}"
        );
    }
}

#[test]
fn split_frontmatter_returns_the_raw_text_whatever_the_root() {
    let cases = [
        (ABSENT, ""),
        (EMPTY, ""),
        (NULL_ROOT, "null\n"),
        (MAPPING, "title: Foo\n"),
        (SEQUENCE_ROOT, "- a\n- b\n"),
        (SCALAR_ROOT, "just a string\n"),
        (INVALID_YAML, "key: [unclosed\n"),
        (TAGGED, "key: !custom value\n"),
    ];
    for (content, frontmatter) in cases {
        assert_eq!(
            YamlFrontmatter.split_frontmatter(content),
            Ok(frontmatter.to_owned()),
            "{content:?}"
        );
    }
}

#[test]
fn split_frontmatter_refuses_an_unterminated_fence() {
    assert_eq!(
        YamlFrontmatter.split_frontmatter(UNTERMINATED),
        Err(FrontmatterError(
            "unterminated frontmatter block".to_owned()
        ))
    );
}
