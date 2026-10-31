from pathlib import Path

import pytest

from tasks.eval.ask_and_report import verdicts
from tasks.eval.ask_and_report.layout import SET
from tasks.eval.ask_and_report.turns import FileState, TurnRecord
from tasks.eval.headless.transcript import Event, Text

SLUG = "llm-agent-planning"
S = SET.as_posix()
ACCEPT = (
    "Reply accept to write reports/how-do-x.md, reject to propose research "
    "instead, or decline to write nothing."
)
PICK = (
    "Reply rewrite 1 (or another listed number) to rewrite that report in "
    "place, new to write reports/how-do-x-2.md, reject to propose research "
    "instead, or decline to write nothing."
)
CONFIRM = (
    f"Reply confirm to append it to outline.md and reopen {SLUG}, or decline."
)
REJECT = (
    "Reply reject to propose research if this does not answer your question."
)
PROPOSAL = (
    "Proposed Round 4:\n"
    "- [ ] How do agents recover from failed steps? — profiles: web, openalex\n"
    "\n" + CONFIRM
)
JUDGED = (
    "Agents plan by decomposition.\n\n"
    "**Coverage:** partial\n"
    "Covered:\n"
    "- planning: Hierarchical Task Planning (tier-1)\n"
    "Gaps:\n"
    "- ranking: Agent planner leaderboard (tier-3 (also tier-1))\n"
    "- recovery: no source\n"
)
OUTLINE = (
    '---\nlast_updated: "2026-10-30T00:00:00+00:00"\n'
    'last_updated_by: "Fixture Author"\n---\n\n## Round 3\n\n- [x] Old\n'
)
APPENDED = (
    OUTLINE.replace("2026-10-30", "2026-11-01").replace(
        "Fixture Author", "Eval Author"
    )
    + "\n## Round 4\n\n"
    "- [ ] How do agents recover from failed steps? — profiles: web, openalex\n"
)
LEADERBOARD = (
    "- [Agent planner leaderboard](https://openalex.org/W3004) — tier-3"
)
REPORT = f"""---
id: "llm-agent-planning-report-how-do-x"
title: "How do x?"
question: "How do x?"
date: "2026-10-30T00:00:00+00:00"
coverage: "partial"
status: "complete"
parent: "work-item:0900"
rounds_covered: 3
---
# How do x?

## Answer
Agents plan.

## Coverage
Planning is covered.

## Gaps
Benchmark ranking rests on one tier-3 source.

## Sources
{LEADERBOARD} (also tier-1) — Journal of Agent Benchmarks
  - doi: 10.5555/leaderboard.2026
  - openalex_id: W3004
"""
FINDING = f"""## Sources
{LEADERBOARD} — Journal of Agent Benchmarks
"""
MANIFEST = """---
status: "synthesised"
report_count: 1
---

## Set
- **Synthesis**: `synthesis.md`
- **Reports**: `reports/`
"""
INITIAL = {
    f"{S}/outline.md": OUTLINE,
    f"{S}/manifest.md": MANIFEST.replace("report_count: 1", "report_count: 0"),
    f"{S}/findings/04-x-openalex.md": FINDING,
    f"{S}/reports/a.md": REPORT,
}


def _files(texts: dict[str, str]) -> dict[str, FileState]:
    return {
        path: FileState(str(hash(text)), text) for path, text in texts.items()
    }


def _turn(
    text: str = "",
    *,
    after: dict[str, str] | None = None,
    before: dict[str, str] | None = None,
    earlier_text: str | None = None,
) -> TurnRecord:
    before_files = _files(before if before is not None else INITIAL)
    earlier: tuple[tuple[Event, ...], ...] = (
        ((Text(earlier_text, by_subagent=False),),)
        if earlier_text is not None
        else ()
    )
    return TurnRecord(
        number=len(earlier) + 1,
        reply=None,
        writes=(),
        events=(Text(text, by_subagent=False),),
        logged=True,
        exit_status="0",
        before=before_files,
        after=_files(after) if after is not None else before_files,
        initial=_files(INITIAL),
        earlier=earlier,
    )


def _judging(validation: str | None = None) -> verdicts.Judging:
    return verdicts.Judging(
        root=Path("/project"), verb="report", validate=lambda path: validation
    )


def _wrote(text: str, **files: str) -> TurnRecord:
    return _turn(
        text, after=INITIAL | {f"{S}/{p}": t for p, t in files.items()}
    )


def _report_turn(report: str = REPORT, manifest: str = MANIFEST) -> TurnRecord:
    after = INITIAL | {
        f"{S}/reports/how-do-x.md": report,
        f"{S}/manifest.md": manifest,
    }
    return _turn("Wrote reports/how-do-x.md (coverage partial).", after=after)


REFUSAL = (
    "report refused: llm-agent-planning is researching; it needs synthesised "
    "or complete. Run conduct llm-agent-planning to finish any outstanding "
    "research, then synthesise llm-agent-planning."
)
GOOD = [
    (
        {
            "refusal": {
                "gate": "status",
                "names": ["researching"],
                "omits": ["synthesis is stale"],
            }
        },
        _turn(f"`{REFUSAL}`"),
    ),
    ({"coverage": "partial"}, _turn(JUDGED)),
    ({"covered": ["PLAN"]}, _turn(JUDGED)),
    (
        {
            "gaps": [
                {"keyword": "rank", "tier": "tier-3 (also tier-1)"},
                {"keyword": "recover"},
            ]
        },
        _turn(JUDGED),
    ),
    (
        {"equivalents": ["reports/a.md"]},
        _turn("Equivalent reports:\n1. reports/a.md — How do x?\n\n" + PICK),
    ),
    ({"equivalents": []}, _turn(JUDGED + ACCEPT)),
    ({"offers": "accept"}, _turn(JUDGED + "\n" + ACCEPT)),
    ({"offers": "pick"}, _turn(PICK)),
    ({"offers": "reject"}, _turn(JUDGED + REJECT)),
    ({"offers": "proposal"}, _turn(JUDGED + PROPOSAL)),
    ({"offers": "none"}, _turn("Nothing written.")),
    ({"writes": ["outline.md"]}, _wrote("", **{"outline.md": APPENDED})),
    ({"writes": []}, _turn("")),
    ({"unchanged": ["reports/*"]}, _wrote("", **{"outline.md": APPENDED})),
    (
        {
            "report": {
                "path": "reports/how-do-x.md",
                "fields": {
                    "coverage": "partial",
                    "rounds_covered": 3,
                    "parent": "work-item:0900",
                },
                "absent": ["relates_to"],
                "sections": ["Answer", "Coverage", "Gaps", "Sources"],
                "no_sections": ["Method"],
                "section_mentions": {"Gaps": ["ranking"]},
            }
        },
        _report_turn(),
    ),
    (
        {
            "report": {"path": "reports/how-do-x.md", "fields": {}},
            "sources": [
                {
                    "marker": "W3004",
                    "tier": "tier-3 (also tier-1)",
                    "block": [
                        ["doi", "10.5555/leaderboard.2026"],
                        ["openalex_id", "W3004"],
                    ],
                }
            ],
        },
        _report_turn(),
    ),
    (
        {
            "manifest": {
                "fields": {"report_count": 1, "status": "synthesised"},
                "lists_reports": True,
            }
        },
        _report_turn(),
    ),
    (
        {
            "proposal": {
                "max_items": 2,
                "profiles": ["web", "openalex"],
                "each_mentions": ["recover", "fail"],
            }
        },
        _turn(JUDGED + PROPOSAL),
    ),
    (
        {"appended_round": 4},
        _turn(
            "Appended Round 4 and reopened llm-agent-planning to researching. "
            "Next: run conduct llm-agent-planning, then synthesise "
            "llm-agent-planning, then report llm-agent-planning How? again.",
            after=INITIAL | {f"{S}/outline.md": APPENDED},
            earlier_text=JUDGED + PROPOSAL,
        ),
    ),
    (
        {"next_steps": True},
        _turn(
            "Appended Round 4 and reopened llm-agent-planning to researching. "
            "Next: run conduct llm-agent-planning, then synthesise "
            "llm-agent-planning, then report llm-agent-planning How? again."
        ),
    ),
    ({"next_steps": False}, _turn("Nothing written.")),
]


@pytest.mark.parametrize(
    ("expect", "record"), GOOD, ids=[next(iter(e)) for e, _ in GOOD]
)
def test_a_good_turn_meets_each_criterion(
    expect: dict, record: TurnRecord
) -> None:
    for criterion in verdicts.parse_criteria(expect):
        assert criterion.judge(record, _judging()) == []


def _fails(expect: dict, record: TurnRecord, judging=None) -> bool:
    return any(
        criterion.judge(record, judging or _judging())
        for criterion in verdicts.parse_criteria(expect)
    )


STATUS = {
    "gate": "status",
    "names": ["researching"],
    "omits": ["synthesis is stale"],
}
REPORT_EXPECT = {"path": "reports/how-do-x.md", "fields": {}}
SOURCES = {
    "report": REPORT_EXPECT,
    "sources": [
        {
            "marker": "W3004",
            "tier": "tier-3 (also tier-1)",
            "block": [
                ["doi", "10.5555/leaderboard.2026"],
                ["openalex_id", "W3004"],
            ],
        }
    ],
}
PLANTED = [
    (
        "refusal-template",
        {"refusal": STATUS},
        _turn("ask refused: other is researching."),
    ),
    (
        "refusal-names",
        {"refusal": STATUS | {"names": ["briefed"]}},
        _turn(REFUSAL),
    ),
    (
        "refusal-omits",
        {"refusal": STATUS | {"omits": ["researching"]}},
        _turn(REFUSAL),
    ),
    (
        "refusal-changed",
        {"refusal": STATUS},
        _wrote(REFUSAL, **{"notes.md": "x"}),
    ),
    ("coverage", {"coverage": "answered"}, _turn(JUDGED)),
    ("coverage-missing", {"coverage": "answered"}, _turn("No judgement.")),
    ("covered-keyword", {"covered": ["benchmark"]}, _turn(JUDGED)),
    ("covered-block", {"covered": ["plan"]}, _turn("Coverage: unanswered")),
    ("gaps-keyword", {"gaps": [{"keyword": "licence"}]}, _turn(JUDGED)),
    (
        "gaps-tier",
        {"gaps": [{"keyword": "rank", "tier": "tier-2"}]},
        _turn(JUDGED),
    ),
    (
        "gaps-ellipsis",
        {"gaps": [{"keyword": "rank"}]},
        _turn(JUDGED + "- rank: {source} (tier text), …\n"),
    ),
    (
        "gaps-or-none",
        {"gaps": [{"keyword": "rank"}]},
        _turn(JUDGED.replace("no source", "sources, or none")),
    ),
    (
        "equivalents",
        {"equivalents": []},
        _turn("Equivalent reports:\n1. reports/a.md — Q\n"),
    ),
    ("offers-final", {"offers": "accept"}, _turn(JUDGED + REJECT)),
    (
        "offers-before-proposal",
        {"offers": "proposal"},
        _turn(ACCEPT + "\n" + PROPOSAL),
    ),
    ("offers-none-prompt", {"offers": "none"}, _turn(REJECT)),
    (
        "offers-none-proposal",
        {"offers": "none"},
        _turn("Proposed Round 4:\n- [ ] Q — profiles: web\nDone."),
    ),
    ("writes-extra", {"writes": []}, _wrote("", **{"outline.md": APPENDED})),
    (
        "writes-missing",
        {"writes": ["outline.md", "manifest.md"]},
        _wrote("", **{"outline.md": APPENDED}),
    ),
    (
        "unchanged",
        {"unchanged": ["reports/*"]},
        _wrote("", **{"reports/a.md": "x"}),
    ),
    (
        "report-path",
        {"report": {"path": "reports/other.md", "fields": {}}},
        _report_turn(),
    ),
    (
        "report-field",
        {"report": REPORT_EXPECT | {"fields": {"coverage": "answered"}}},
        _report_turn(),
    ),
    (
        "report-absent",
        {"report": REPORT_EXPECT | {"absent": ["parent"]}},
        _report_turn(),
    ),
    (
        "report-sections",
        {"report": REPORT_EXPECT | {"sections": ["Method"]}},
        _report_turn(),
    ),
    (
        "report-no-sections",
        {"report": REPORT_EXPECT | {"no_sections": ["Gaps"]}},
        _report_turn(),
    ),
    (
        "report-mentions",
        {"report": REPORT_EXPECT | {"section_mentions": {"Gaps": ["recover"]}}},
        _report_turn(),
    ),
    (
        "report-kept",
        {"report": {"path": "reports/a.md", "fields": {}, "kept": ["date"]}},
        _wrote(
            "", **{"reports/a.md": REPORT.replace("2026-10-30T", "2026-11-01T")}
        ),
    ),
    ("sources-none", SOURCES, _report_turn(REPORT.replace("W3004)", "W9999)"))),
    (
        "sources-two",
        SOURCES,
        _report_turn(
            REPORT + "- [Again](https://openalex.org/W3004/x) — tier-1\n"
        ),
    ),
    (
        "sources-tier",
        SOURCES,
        _report_turn(REPORT.replace(" (also tier-1)", "")),
    ),
    (
        "sources-block",
        SOURCES,
        _report_turn(REPORT.replace("  - openalex_id: W3004\n", "")),
    ),
    (
        "sources-block-order",
        SOURCES,
        _report_turn(
            REPORT.replace(
                "  - doi: 10.5555/leaderboard.2026\n  - openalex_id: W3004\n",
                "  - openalex_id: W3004\n  - doi: 10.5555/leaderboard.2026\n",
            )
        ),
    ),
    ("sources-slot", SOURCES, _report_turn(REPORT + "  - {source-metadata}\n")),
    (
        "sources-added",
        SOURCES,
        _report_turn(
            REPORT
            + "- [Made up](https://made-up.example/x) — tier-1 — "
            + "made-up.example\n"
        ),
    ),
    (
        "manifest-field",
        {"manifest": {"fields": {"report_count": 2}, "lists_reports": True}},
        _report_turn(),
    ),
    (
        "manifest-lists",
        {"manifest": {"fields": {}, "lists_reports": True}},
        _report_turn(
            manifest=MANIFEST.replace("- **Reports**: `reports/`\n", "")
        ),
    ),
    ("proposal-max", {"proposal": {"max_items": 0}}, _turn(PROPOSAL)),
    ("proposal-profiles", {"proposal": {"profiles": ["web"]}}, _turn(PROPOSAL)),
    (
        "proposal-mentions",
        {"proposal": {"each_mentions": ["gpu"]}},
        _turn(PROPOSAL),
    ),
    ("proposal-missing", {"proposal": {}}, _turn(JUDGED)),
    (
        "appended-revises",
        {"appended_round": 4},
        _turn(
            after=INITIAL
            | {f"{S}/outline.md": APPENDED.replace("[x] Old", "[x] New")},
            earlier_text=PROPOSAL,
        ),
    ),
    (
        "appended-two-rounds",
        {"appended_round": 4},
        _turn(
            after=INITIAL | {f"{S}/outline.md": APPENDED + "\n## Round 5\n"},
            earlier_text=PROPOSAL,
        ),
    ),
    (
        "appended-stale-fields",
        {"appended_round": 4},
        _turn(
            after=INITIAL
            | {f"{S}/outline.md": OUTLINE + APPENDED[len(OUTLINE) :]},
            earlier_text=PROPOSAL,
        ),
    ),
    (
        "appended-items",
        {"appended_round": 4},
        _turn(
            after=INITIAL | {f"{S}/outline.md": APPENDED},
            earlier_text=PROPOSAL.replace("recover from", "skip"),
        ),
    ),
    ("next-steps-missing", {"next_steps": True}, _turn("Nothing written.")),
    (
        "next-steps-present",
        {"next_steps": False},
        _turn(
            "Appended Round 4 and reopened llm-agent-planning to researching. "
            "Next: run conduct llm-agent-planning, then synthesise "
            "llm-agent-planning, then report llm-agent-planning How? again."
        ),
    ),
]


@pytest.mark.parametrize(
    ("expect", "record"),
    [(expect, record) for _, expect, record in PLANTED],
    ids=[name for name, _, _ in PLANTED],
)
def test_a_planted_failure_fails_its_criterion(
    expect: dict, record: TurnRecord
) -> None:
    assert _fails(expect, record)


def test_a_report_that_fails_validation_fails() -> None:
    assert _fails(
        {"report": REPORT_EXPECT}, _report_turn(), _judging("BAD-STATUS")
    )


@pytest.mark.parametrize(
    "written",
    [
        "Coverage: answered",
        "**Coverage: answered**",
        "> Coverage: `answered`",
        "```text\nCoverage: answered\n```",
    ],
)
def test_a_formatted_fixed_line_still_matches(written: str) -> None:
    assert not _fails({"coverage": "answered"}, _turn(written))


@pytest.mark.parametrize(
    ("key", "judgement"),
    [
        ("refusal", False),
        ("coverage", True),
        ("covered", True),
        ("gaps", True),
        ("equivalents", True),
        ("offers", True),
        ("writes", False),
        ("unchanged", False),
        ("report", False),
        ("sources", True),
        ("manifest", False),
        ("proposal", True),
        ("appended_round", False),
        ("next_steps", False),
    ],
)
def test_each_criterion_type_says_whether_it_is_a_judgement(
    key: str, judgement: bool
) -> None:
    assert verdicts.CRITERIA[key].is_judgement is judgement


@pytest.mark.parametrize(
    "expect",
    [
        {"colour": "red"},
        {"offers": "maybe"},
        {"sources": SOURCES["sources"]},
        {"refusal": {"gate": "closed"}},
        {"report": {"path": "r.md", "colour": "red"}},
    ],
    ids=[
        "unknown",
        "bad-offer",
        "sources-without-report",
        "bad-gate",
        "report-key",
    ],
)
def test_a_malformed_expectation_raises(expect: dict) -> None:
    with pytest.raises((ValueError, TypeError, KeyError)):
        verdicts.parse_criteria(expect)
