import fcntl
import json

import pytest

from tests.integration.support.characterisation import Mask

SET = "meta/research/topics/s"
LEDGER = f"{SET}/.conduct-run.json"
ARXIV_LOCK = ".accelerator/tmp/research/arxiv.lock"

_BRIEF = (
    '---\ntype: "topic-research"\nkind: "brief"\n'
    'source_profiles: ["web"]\n---\n'
)
_OUTLINE = (
    '---\ntype: "topic-research"\nkind: "outline"\n---\n\n'
    "- [x] A?\n- [x] B?\n- [ ] C?\n"
)


def _provenance(stem: str, kind: str, title: str) -> str:
    return (
        f'---\ntype: "topic-research"\nid: "{stem}"\ntitle: "{title}"\n'
        'date: "2026-09-27T00:00:00+00:00"\nauthor: "Fixture Author"\n'
        'producer: "research-topic"\nstatus: "complete"\n'
        f'kind: "{kind}"\nround: 1\nsource_profile: "web"\n'
    )


_TRAILER = (
    'tags: ["research"]\nlast_updated: "2026-09-27T00:00:00+00:00"\n'
    'last_updated_by: "Fixture Author"\nschema_version: 1\n---\n'
)


def _finding(stem: str, question: str) -> str:
    return (
        _provenance(stem, "finding", "A")
        + f"question: {json.dumps(question)}\ndepth: 2\n"
        + _TRAILER
    )


def _level_note(stem: str, question: str, lineage: str, extra: str) -> str:
    return (
        _provenance(f"s.{stem}.{lineage}", "level-note", "A note")
        + f"depth: 2\nquestion: {json.dumps(question)}\n"
        + f"level: {lineage.split('-', maxsplit=1)[0]}\n"
        + extra
        + _TRAILER
        + "\n# A note\n"
    )


@pytest.fixture
def topic(repository):
    repository.write(f"{SET}/manifest.md", "---\n---\n")
    repository.write(f"{SET}/brief.md", _BRIEF)
    repository.write(f"{SET}/outline.md", _OUTLINE)
    repository.write(f"{SET}/findings/01-a-web.md", _finding("01-a-web", "A?"))
    repository.write(
        f"{SET}/findings/01-a-web.levels/1.md",
        _level_note("01-a-web", "A?", "1", 'follow_ups: ["Deeper?"]\n'),
    )
    repository.write(
        f"{SET}/findings/04-c-web.levels/1.md",
        _level_note("04-c-web", "C?", "1", 'follow_ups: ["Further?"]\n'),
    )
    repository.write(
        f"{SET}/findings/04-c-web.levels/2-1.md",
        _level_note("04-c-web", "Further?", "2-1", ""),
    )
    repository.write(
        f"{SET}/findings/02-b-web.md",
        '---\ntype: "topic-research"\nkind: "finding"\n'
        'question: "B?"\nsource_profile: "web"\nstatus: "draft"\n---\n',
    )
    repository.write("profiles/web-profile/SKILL.md", "web\n")
    repository.commit()
    return repository


def _research(run, binaries, repository, *args, env=None):
    return run(
        binaries.subbinary("research"),
        *args,
        cwd=repository.root,
        env=env or {},
    )


def _outstanding(run, binaries, topic, *extra):
    return _research(
        run,
        binaries,
        topic,
        "topic",
        "outstanding",
        "s",
        "--profiles-dir",
        str(topic.root / "profiles"),
        "--depth",
        "2",
        *extra,
    )


def _ledger_observation(result, topic):
    ledger = topic.root / LEDGER
    if not ledger.exists():
        return result.with_observation(f"{LEDGER} is absent")
    return result.with_observation(f"{LEDGER}:\n{ledger.read_text().rstrip()}")


def _seed_ledger(topic, run_id="r1"):
    topic.write(
        LEDGER,
        json.dumps(
            {
                "run": run_id,
                "pending": {"number": 0, "spawns": []},
                "claims": {"A?": 1},
                "attempted": [],
                "just_acknowledged": [],
                "seen": {"notes": {}, "answered": []},
            }
        )
        + "\n",
    )


def test_outstanding_validates_findings_and_level_notes(
    topic, binaries, run, matches_golden
):
    matches_golden(_outstanding(run, binaries, topic))


def test_a_started_run_writes_the_ledger(topic, binaries, run, matches_golden):
    result = _ledger_observation(
        _outstanding(run, binaries, topic, "--start"), topic
    )
    minted = json.loads((topic.root / LEDGER).read_text())["run"]

    matches_golden(result, [Mask(minted, "RUN")])


def test_a_continued_run_reads_and_rewrites_the_ledger(
    topic, binaries, run, matches_golden
):
    _seed_ledger(topic)

    result = _outstanding(run, binaries, topic, "--run", "r1", "--spawned", "0")

    matches_golden(_ledger_observation(result, topic))


def test_a_superseded_run_is_refused(topic, binaries, run, matches_golden):
    _seed_ledger(topic, "r2")

    result = _outstanding(run, binaries, topic, "--run", "r1")

    matches_golden(_ledger_observation(result, topic))


def test_ending_a_run_removes_its_ledger(topic, binaries, run, matches_golden):
    _seed_ledger(topic)

    result = _research(
        run, binaries, topic, "topic", "end-run", "s", "--run", "r1"
    )

    matches_golden(_ledger_observation(result, topic))


def test_arxiv_fetch_under_lock_contention(
    topic, binaries, run, matches_golden, fixture_root
):
    lock = topic.root / ARXIV_LOCK
    lock.parent.mkdir(parents=True, exist_ok=True)
    clock_log = fixture_root / "clock.log"
    with lock.open("a") as held:
        fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
        result = _research(
            run,
            binaries,
            topic,
            "fetch",
            "arxiv",
            "search",
            "x",
            env={
                "ACCELERATOR_RESEARCH_TEST_CLOCK_LOG": str(clock_log),
                "ACCELERATOR_RESEARCH_TEST_CLOCK_EPOCH": "1800000000000",
            },
        )

    contention = topic.root / ".accelerator/tmp/research/arxiv-contention.log"
    matches_golden(
        result.with_observation(
            f"contention log lines: {len(contention.read_text().splitlines())}"
            if contention.exists()
            else "contention log is absent"
        )
    )
