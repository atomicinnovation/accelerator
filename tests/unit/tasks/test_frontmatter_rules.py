"""Tests for the shared frontmatter emission rules in
``tasks/lint/frontmatter_rules.py``."""

from tasks.lint import frontmatter_rules as fr


def test_base_fields_exclude_producer_and_status() -> None:
    assert "producer" not in fr.BASE_FIELDS
    assert "status" not in fr.BASE_FIELDS
    assert fr.BASE_FIELDS[0] == "type"
    assert "schema_version" in fr.BASE_FIELDS


def test_cardinality_classifies_single_list_and_unknown() -> None:
    assert fr.linkage_cardinality("parent") == "single"
    assert fr.linkage_cardinality("target") == "single"
    assert fr.linkage_cardinality("blocks") == "list"
    assert fr.linkage_cardinality("relates_to") == "list"
    assert fr.linkage_cardinality("nonsense") == ""


def test_is_linkage_key_covers_the_vocabulary() -> None:
    for key in fr.LINKAGE_VOCABULARY:
        assert fr.is_linkage_key(key)
    assert not fr.is_linkage_key("kind")


def test_superseded_by_is_a_guard_in_the_vocabulary() -> None:
    # No template carries it, but the closed-set check must reject one that adds
    # it, so it stays in the vocabulary.
    assert "superseded_by" in fr.LINKAGE_VOCABULARY


def test_id_quoted_regex_requires_a_quoted_scalar() -> None:
    assert fr.ID_QUOTED_RE.match('id: "0001"')
    assert fr.ID_QUOTED_RE.match('id: "PROJ-1"  # trailing comment')
    assert not fr.ID_QUOTED_RE.match("id: 0001")
    assert not fr.ID_QUOTED_RE.match("id: unquoted")


def test_schema_version_regex_requires_the_bare_integer_one() -> None:
    assert fr.SCHEMA_VERSION_RE.match("schema_version: 1")
    assert not fr.SCHEMA_VERSION_RE.match('schema_version: "1"')
    assert not fr.SCHEMA_VERSION_RE.match("schema_version: 2")


def test_typed_ref_regex_accepts_type_id_and_rejects_bare_and_path() -> None:
    assert fr.TYPED_REF_RE.match("work-item:0042")
    assert fr.TYPED_REF_RE.match("adr:ADR-0001")
    assert fr.TYPED_REF_RE.match("plan:2026-01-01-changelog-1.21.0")
    assert not fr.TYPED_REF_RE.match("0042")
    assert not fr.TYPED_REF_RE.match("meta/work/0042")
    assert not fr.TYPED_REF_RE.match("bogus-type:0042")


def test_optional_extras_are_omit_when_empty() -> None:
    assert "external_id" in fr.OPTIONAL_EXTRAS
    assert "reviewer" in fr.OPTIONAL_EXTRAS
    assert "work_item_id" in fr.OPTIONAL_EXTRAS


def test_schema_columns_ok_matches_order_and_tolerates_extension() -> None:
    exact = "\t".join(fr.SCHEMA_COLUMNS)
    assert fr.schema_columns_ok(exact)
    assert fr.schema_columns_ok(exact + "\textra_future_column")
    assert fr.schema_columns_ok(exact + "\r")
    assert not fr.schema_columns_ok("type\ttemplate")
    assert not fr.schema_columns_ok("")
