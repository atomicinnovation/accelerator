from tasks.eval.documents.frontmatter import read_fields


def test_each_field_is_read_as_json_where_it_parses() -> None:
    document = '---\ntitle: "How do x?"\nround_count: 3\n---\n\nBody\n'

    assert read_fields(document) == {"title": "How do x?", "round_count": 3}


def test_a_value_that_is_not_json_is_read_as_written() -> None:
    assert read_fields("---\nstatus: draft\n---\n") == {"status": "draft"}


def test_the_block_ends_at_the_closing_fence() -> None:
    document = "---\nstatus: draft\n---\n\nnote: in the body\n"

    assert read_fields(document) == {"status": "draft"}


def test_a_document_not_opening_with_a_fence_has_no_fields() -> None:
    document = "Preamble\n---\nstatus: draft\n---\n"

    assert read_fields(document) == {}


def test_an_empty_document_has_no_fields() -> None:
    assert read_fields("") == {}
