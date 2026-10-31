import json
from pathlib import Path

import pytest

from tasks.eval.ask_and_report.mutations import (
    Append,
    DropLine,
    Mutation,
    Remove,
    Rename,
    Replace,
    SetField,
    parse_mutation,
)
from tasks.eval.ask_and_report.preflight import gate_of
from tasks.eval.ask_and_report.scenarios import (
    Scenario,
    Setup,
    load_scenario_files,
)

MANIFEST = """---
type: "topic-research"
status: "synthesised"
finding_count: 10
---

# Manifest
"""


@pytest.fixture
def set_root(tmp_path: Path) -> Path:
    (tmp_path / "findings").mkdir()
    (tmp_path / "manifest.md").write_text(MANIFEST)
    (tmp_path / "findings/01-a-web.md").write_text(
        "source_profile: web\nround: 1\nround: 1\n"
    )
    return tmp_path


def test_set_rewrites_one_frontmatter_field(set_root: Path) -> None:
    SetField("manifest.md", "finding_count", 9).apply(set_root)

    assert "finding_count: 9\n" in (set_root / "manifest.md").read_text()


def test_set_writes_a_string_value_quoted(set_root: Path) -> None:
    SetField("manifest.md", "status", "briefed").apply(set_root)

    assert 'status: "briefed"\n' in (set_root / "manifest.md").read_text()


def test_remove_deletes_the_file(set_root: Path) -> None:
    Remove("findings/01-a-web.md").apply(set_root)

    assert not (set_root / "findings/01-a-web.md").exists()


def test_rename_moves_the_file(set_root: Path) -> None:
    Rename("findings/01-a-web.md", "findings/.01-a-web.md.invalid").apply(
        set_root
    )

    assert not (set_root / "findings/01-a-web.md").exists()
    assert (set_root / "findings/.01-a-web.md.invalid").exists()


def test_drop_line_deletes_the_one_line_starting_so(set_root: Path) -> None:
    DropLine("findings/01-a-web.md", "source_profile:").apply(set_root)

    assert (set_root / "findings/01-a-web.md").read_text() == (
        "round: 1\nround: 1\n"
    )


def test_append_adds_the_text_at_the_end(set_root: Path) -> None:
    Append("manifest.md", "\n## Round 4\n").apply(set_root)

    assert (set_root / "manifest.md").read_text() == MANIFEST + "\n## Round 4\n"


def test_replace_swaps_the_one_occurrence(set_root: Path) -> None:
    Replace("manifest.md", "# Manifest", "# Changed").apply(set_root)

    assert (set_root / "manifest.md").read_text().endswith("# Changed\n")


@pytest.mark.parametrize(
    "mutation",
    [
        SetField("absent.md", "status", "briefed"),
        Remove("absent.md"),
        Rename("absent.md", "elsewhere.md"),
        DropLine("absent.md", "status:"),
        Append("absent.md", "text"),
        Replace("absent.md", "old", "new"),
    ],
    ids=lambda mutation: type(mutation).__name__,
)
def test_a_mutation_naming_a_missing_file_raises(
    set_root: Path, mutation: Mutation
) -> None:
    with pytest.raises(FileNotFoundError):
        mutation.apply(set_root)


def test_set_naming_a_missing_field_raises(set_root: Path) -> None:
    with pytest.raises(ValueError, match="round_count"):
        SetField("manifest.md", "round_count", 2).apply(set_root)


@pytest.mark.parametrize(
    "mutation",
    [
        DropLine("findings/01-a-web.md", "depth:"),
        DropLine("findings/01-a-web.md", "round:"),
        Replace("findings/01-a-web.md", "depth: 1", "depth: 2"),
        Replace("findings/01-a-web.md", "round: 1", "round: 2"),
    ],
    ids=["drop-none", "drop-two", "replace-none", "replace-two"],
)
def test_a_line_mutation_needs_exactly_one_match(
    set_root: Path, mutation: DropLine | Replace
) -> None:
    with pytest.raises(ValueError, match="exactly one"):
        mutation.apply(set_root)


@pytest.mark.parametrize(
    ("data", "parsed"),
    [
        (
            {"set": "manifest.md", "field": "round_count", "value": 2},
            SetField("manifest.md", "round_count", 2),
        ),
        ({"remove": "a.md"}, Remove("a.md")),
        ({"rename": "a.md", "to": "b.md"}, Rename("a.md", "b.md")),
        (
            {"drop_line": "a.md", "starting": "x:"},
            DropLine("a.md", "x:"),
        ),
        ({"append": "a.md", "text": "t"}, Append("a.md", "t")),
        (
            {"replace": "a.md", "old": "o", "new": "n"},
            Replace("a.md", "o", "n"),
        ),
    ],
)
def test_each_mutation_key_parses_into_its_type(
    data: dict[str, object], parsed: object
) -> None:
    assert parse_mutation(data) == parsed


@pytest.mark.parametrize(
    "data",
    [
        {"truncate": "a.md"},
        {"remove": "a.md", "to": "b.md"},
        {"rename": "a.md"},
        {},
    ],
    ids=["unknown-key", "extra-key", "missing-key", "empty"],
)
def test_a_mutation_of_no_known_shape_raises(data: dict[str, object]) -> None:
    with pytest.raises(ValueError):
        parse_mutation(data)


@pytest.mark.parametrize(
    ("verdict", "gate"),
    [
        ({"consultable": True, "findings": [], "reports": []}, "open"),
        ({"consultable": False, "gate": "status", "status": "x"}, "status"),
        (
            {"consultable": False, "gate": "freshness", "mismatches": ["m"]},
            "freshness",
        ),
        (
            {
                "consultable": False,
                "gate": "conducted",
                "unanswered": [{"question": "q", "profile": "web"}],
                "held_back": [{"question": "h", "reasons": ["r"]}],
            },
            "conducted",
        ),
        (
            {
                "consultable": False,
                "gate": "conducted",
                "unanswered": [],
                "held_back": [{"question": "h", "reasons": ["r"]}],
            },
            "held_back",
        ),
    ],
)
def test_gate_of_names_the_gate_a_verdict_stops_at(
    verdict: dict[str, object], gate: str
) -> None:
    assert gate_of(verdict) == gate


SCENARIO = {
    "name": "g1-briefed-ask",
    "verb": "ask",
    "question": "How do LLM agents plan multi-step tasks?",
    "times": 1,
    "setup": {
        "variant": "base",
        "mutations": [
            {"set": "manifest.md", "field": "status", "value": "briefed"}
        ],
    },
    "gate": "status",
}


def _scenario_file(directory: Path, name: str, *scenarios: object) -> None:
    (directory / name).write_text(
        json.dumps({"family": "G1", "title": "t", "scenarios": scenarios})
    )


def test_a_scenario_parses_into_typed_values(tmp_path: Path) -> None:
    _scenario_file(tmp_path, "g1.json", SCENARIO)

    (scenario_file,) = load_scenario_files(tmp_path)

    assert scenario_file.family == "G1"
    assert scenario_file.scenarios == (
        Scenario(
            name="g1-briefed-ask",
            verb="ask",
            question="How do LLM agents plan multi-step tasks?",
            times=1,
            setup=Setup(
                variant="base",
                mutations=(SetField("manifest.md", "status", "briefed"),),
            ),
            gate="status",
        ),
    )


def test_a_setup_carries_breadth_templates_and_skips(tmp_path: Path) -> None:
    setup = {
        "variant": "equivalence",
        "breadth": 2,
        "templates": ["user", "config"],
        "skips": ["Which agent frameworks document a planning API?"],
    }
    _scenario_file(tmp_path, "p1.json", SCENARIO | {"setup": setup})

    ((scenario,),) = (f.scenarios for f in load_scenario_files(tmp_path))

    assert scenario.setup == Setup(
        variant="equivalence",
        breadth=2,
        templates=("user", "config"),
        skips=("Which agent frameworks document a planning API?",),
    )


@pytest.mark.parametrize(
    "scenario",
    [
        SCENARIO | {"gate": "closed"},
        SCENARIO | {"verb": "summarise"},
        SCENARIO | {"turns": []},
        SCENARIO | {"setup": {"variant": "base", "colour": "red"}},
        SCENARIO | {"setup": {"variant": "elsewhere"}},
        SCENARIO | {"setup": {"variant": "base", "templates": ["team"]}},
        SCENARIO | {"setup": {"variant": "base", "mutations": [{"x": "y"}]}},
        SCENARIO | {"times": "three"},
        {key: value for key, value in SCENARIO.items() if key != "gate"},
    ],
    ids=[
        "unknown-gate",
        "unknown-verb",
        "unknown-key",
        "unknown-setup-key",
        "unknown-variant",
        "unknown-template-tier",
        "unknown-mutation",
        "non-integer-times",
        "missing-gate",
    ],
)
def test_a_malformed_scenario_raises_at_load(
    tmp_path: Path, scenario: dict[str, object]
) -> None:
    _scenario_file(tmp_path, "g1.json", scenario)

    with pytest.raises((ValueError, TypeError, KeyError)):
        load_scenario_files(tmp_path)


def test_a_name_used_twice_across_files_raises(tmp_path: Path) -> None:
    _scenario_file(tmp_path, "g1.json", SCENARIO)
    _scenario_file(tmp_path, "g2.json", SCENARIO)

    with pytest.raises(ValueError, match="g1-briefed-ask"):
        load_scenario_files(tmp_path)

