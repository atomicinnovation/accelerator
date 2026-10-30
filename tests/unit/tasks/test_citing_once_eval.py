import json
from pathlib import Path

import pytest

from tasks.eval.citing_once.fixtures import Composition, load_fixtures
from tasks.eval.citing_once.logs import (
    composer_log_failures,
    synthesise_log_failures,
    tool_calls,
)
from tasks.eval.citing_once.projects import build_project
from tasks.eval.citing_once.rule import key_order
from tasks.eval.citing_once.sessions import session_command
from tasks.eval.citing_once.sources import parse_sources
from tasks.eval.citing_once.verdicts import (
    criterion_failures,
    entry_differences,
    isolation_conflicts,
    predict,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
RULE = REPO_ROOT / "skills/research/outputters/citing-once/SKILL.md"
FIXTURES = RULE.parent / "evals"
SEED = REPO_ROOT / "cli/corpus-cli/tests/fixtures/topic-research-deepened-set"
RECOMMENDED = key_order(RULE.read_text())
FIXTURE_SET = load_fixtures(FIXTURES)
COMPOSITIONS = [
    composition
    for fixture in FIXTURE_SET
    for composition in fixture.compositions
]
EACH_COMPOSITION = pytest.mark.parametrize(
    "composition", COMPOSITIONS, ids=[c.name for c in COMPOSITIONS]
)


def _composition(name: str) -> Composition:
    return next(c for c in COMPOSITIONS if c.name == name)


def test_the_fixtures_plan_every_acceptance_session() -> None:
    sessions = {
        fixture.family: sum(c.session_count for c in fixture.compositions)
        for fixture in FIXTURE_SET
    }
    assert sessions == {
        "C1": 3,
        "C2": 3,
        "C3": 3,
        "C4": 9,
        "C5": 9,
        "S1": 3,
        "S2": 6,
    }


def test_the_fixtures_cover_every_criterion() -> None:
    criteria = {
        fixture.family: sum(len(c.criteria) for c in fixture.compositions)
        for fixture in FIXTURE_SET
    }
    assert criteria == {
        "C1": 7,
        "C2": 6,
        "C3": 17,
        "C4": 5,
        "C5": 7,
        "S1": 11,
        "S2": 1,
    }


@EACH_COMPOSITION
def test_no_two_criteria_share_a_source(composition: Composition) -> None:
    assert isolation_conflicts(composition) == []


@EACH_COMPOSITION
def test_the_rule_predicts_every_expectation(composition: Composition) -> None:
    for order in composition.orders:
        predicted = predict(composition, order, RECOMMENDED)
        assert criterion_failures(composition, predicted) == []


@EACH_COMPOSITION
def test_every_order_predicts_the_same_entries(
    composition: Composition,
) -> None:
    predictions = {
        tuple(predict(composition, order, RECOMMENDED))
        for order in composition.orders
    }
    assert len(predictions) == 1


def test_a_shared_source_across_criteria_is_a_conflict() -> None:
    composition = Composition.from_json(
        {
            "name": "clash",
            "carrier": "composer",
            "times": 1,
            "criteria": [
                {
                    "id": "c01",
                    "statement": "first",
                    "markers": ["c01.example"],
                    "citations": [
                        {
                            "in": "1",
                            "entry": [
                                "- [A](https://c01.example/a) — tier-2 — x",
                                "  - doi: 10.x/shared",
                            ],
                        }
                    ],
                    "expect": [{}],
                },
                {
                    "id": "c02",
                    "statement": "second",
                    "markers": ["c02.example"],
                    "citations": [
                        {
                            "in": "1",
                            "entry": [
                                "- [B](https://c02.example/b) — tier-2 — x",
                                "  - doi: 10.X/SHARED",
                            ],
                        }
                    ],
                    "expect": [{}],
                },
            ],
        }
    )
    assert isolation_conflicts(composition) == ["c01 and c02 share a source"]


def _predicted_document(composition: Composition) -> str:
    entries = predict(composition, composition.orders[0], RECOMMENDED)
    return "## Sources\n" + "".join(entry.render() for entry in entries)


def test_the_predicted_entries_pass() -> None:
    composition = _composition("c3")
    written = parse_sources(_predicted_document(composition))
    predicted = predict(composition, composition.orders[0], RECOMMENDED)
    assert entry_differences(predicted, written) == []
    assert criterion_failures(composition, written) == []


@pytest.mark.parametrize(
    ("plant", "failing"),
    [
        (
            lambda doc: doc.replace(
                "accessed: 2026-10-01", "accessed: 2026-10-02"
            ),
            "c10",
        ),
        (
            lambda doc: (
                doc
                + "- [Notifier](c11.example/notifier) — tier-2 — c11.example\n"
            ),
            "c11",
        ),
        (
            lambda doc: doc + "- [Extra](https://c07.example/x) — tier-1 — x\n",
            "c07",
        ),
    ],
    ids=["changed-accessed", "duplicated-entry", "added-source"],
)
def test_a_planted_change_fails(plant, failing: str) -> None:
    composition = _composition("c3")
    written = parse_sources(plant(_predicted_document(composition)))
    predicted = predict(composition, composition.orders[0], RECOMMENDED)
    assert entry_differences(predicted, written) != []
    assert any(
        failure.startswith(f"{failing}:")
        for failure in criterion_failures(composition, written)
    )


def _event(tool: str, parent: str | None = None, **tool_input) -> str:
    return json.dumps(
        {
            "type": "assistant",
            "parent_tool_use_id": parent,
            "message": {
                "content": [
                    {
                        "type": "tool_use",
                        "id": f"{tool}-{len(tool_input)}",
                        "name": tool,
                        "input": tool_input,
                    }
                ]
            },
        }
    )


def _log(*events: str) -> list:
    return tool_calls("\n".join(events))


def test_a_composer_that_reads_the_rule_and_runs_nothing_passes() -> None:
    log = _log(
        _event("Agent", subagent_type="accelerator:composer"),
        _event("Read", parent="spawn", file_path=str(RULE)),
        _event("Write", parent="spawn", file_path="/p/finding.md"),
    )
    assert composer_log_failures(log, RULE) == []


def test_a_composer_that_runs_a_command_fails() -> None:
    log = _log(
        _event("Agent", subagent_type="accelerator:composer"),
        _event("Read", parent="spawn", file_path=str(RULE)),
        _event("Bash", parent="spawn", command="ls"),
    )
    assert composer_log_failures(log, RULE) == ["a Bash call was made"]


def test_a_composer_that_never_reads_the_rule_fails() -> None:
    log = _log(_event("Agent", subagent_type="accelerator:composer"))
    assert composer_log_failures(log, RULE) == [
        "the composer never read the rule"
    ]


def test_synthesise_reading_the_rule_before_writing_passes() -> None:
    log = _log(
        _event("Bash", command="accelerator corpus resolve x"),
        _event("Read", file_path=str(RULE)),
        _event("Write", file_path="/p/synthesis.md"),
    )
    assert synthesise_log_failures(log, RULE) == []


def test_synthesise_reading_the_rule_with_cat_fails() -> None:
    log = _log(
        _event("Bash", command=f"cat {RULE}"),
        _event("Write", file_path="/p/synthesis.md"),
    )
    assert synthesise_log_failures(log, RULE) == [
        "the rule was read with Bash",
        "the rule was not read with Read before synthesis.md was written",
    ]


def test_a_command_naming_a_citing_once_slug_does_not_read_the_rule() -> None:
    log = _log(
        _event("Bash", command="accelerator corpus resolve citing-once-s2"),
        _event("Read", file_path=str(RULE)),
        _event("Write", file_path="/p/synthesis.md"),
    )
    assert synthesise_log_failures(log, RULE) == []


def test_synthesise_spawning_an_agent_fails() -> None:
    log = _log(
        _event("Read", file_path=str(RULE)),
        _event("Agent", subagent_type="general-purpose"),
        _event("Write", file_path="/p/synthesis.md"),
    )
    assert synthesise_log_failures(log, RULE) == ["an agent was spawned"]


def test_a_composer_project_places_each_citation_in_its_note(
    tmp_path: Path,
) -> None:
    composition = _composition("c5")
    order = composition.orders[1]
    project = build_project(composition, order, tmp_path, SEED)
    assert [path.stem for path in project.inputs] == list(order)
    root_note = project.inputs[order.index("1")].read_text()
    assert [c.url for c in parse_sources(root_note)][:2] == [
        "https://event-driven.io/en/ordering_in_postgres_outbox/",
        "https://github.com/eugene-khyst/postgresql-event-sourcing",
    ]
    assert project.output.name == "01-cited-sources-web.md"
    assert not project.output.exists()


def test_a_synthesise_project_indexes_findings_in_order(
    tmp_path: Path,
) -> None:
    composition = _composition("s2")
    project = build_project(composition, ("b", "a"), tmp_path, SEED)
    assert [path.name for path in project.inputs] == [
        "01-focus-b-web.md",
        "02-focus-a-web.md",
    ]
    assert project.output.name == "synthesis.md"
    manifest = (project.set_root / "manifest.md").read_text()
    assert 'status: "researching"' in manifest
    assert "finding_count: 2" in manifest


def test_a_session_loads_the_repository_and_not_the_installed_plugin(
    tmp_path: Path,
) -> None:
    command = session_command("Reply OK.", REPO_ROOT)
    settings = json.loads(command[command.index("--settings") + 1])
    assert settings["enabledPlugins"] == {
        "accelerator@atomic-innovation-prerelease": False
    }
    assert command[command.index("--plugin-dir") + 1] == str(REPO_ROOT)
    assert command[command.index("--output-format") + 1] == "stream-json"


def test_each_planned_session_has_its_own_directory(tmp_path: Path) -> None:
    from tasks.eval.citing_once.runs import plan_sessions

    plans = plan_sessions(FIXTURE_SET, tmp_path, families=())
    assert len(plans) == 36
    assert len({plan.directory for plan in plans}) == 36
    assert plans[0].directory == tmp_path / "C1" / "c1" / "order-1-run-1"


def test_a_family_filter_plans_only_that_family(tmp_path: Path) -> None:
    from tasks.eval.citing_once.runs import plan_sessions

    plans = plan_sessions(FIXTURE_SET, tmp_path, families=("S2",))
    assert [plan.directory.name for plan in plans] == [
        "order-1-run-1",
        "order-1-run-2",
        "order-1-run-3",
        "order-2-run-1",
        "order-2-run-2",
        "order-2-run-3",
    ]


def test_a_re_check_judges_only_sessions_on_disk(tmp_path: Path) -> None:
    from tasks.eval.citing_once.runs import plan_sessions, sessions_on_disk

    plans = plan_sessions(FIXTURE_SET, tmp_path, families=("S2",))
    (plans[0].directory / "project").mkdir(parents=True)
    (plans[0].directory / "project.json").write_text("{}")
    assert sessions_on_disk(plans) == [plans[0]]
