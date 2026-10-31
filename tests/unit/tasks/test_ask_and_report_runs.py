import json
import subprocess
from pathlib import Path

import pytest

from tasks.eval.ask_and_report import runs
from tasks.eval.ask_and_report.scenarios import load_scenario_files

REPO_ROOT = Path(__file__).resolve().parents[3]
EVALS = REPO_ROOT / "skills/research/research-topic/evals"
RULE = REPO_ROOT / "skills/research/outputters/citing-once/SKILL.md"
FILES = load_scenario_files(EVALS / "scenarios")
SLUG = "llm-agent-planning"
BRIEFED = (
    f"ask refused: {SLUG} is briefed; it needs synthesised or complete. "
    f"Run outline {SLUG}, then conduct {SLUG}, then synthesise {SLUG}."
)
ANSWERED = (
    "Agents plan by decomposition.\n\nCoverage: answered\nCovered:\n"
    "- planning: Hierarchical Task Planning (tier-1)\n\n"
    "Reply accept to write reports/how-do-llm-agents-plan-multi-step-tasks.md,"
    " reject to propose research instead, or decline to write nothing."
)


def _line(record: dict) -> str:
    return json.dumps(record)


def _log(*, text: str = "", read_rule: bool = False, bash: str = "") -> str:
    lines = [
        _line(
            {
                "type": "system",
                "subtype": "init",
                "session_id": "s",
                "model": "claude-test",
                "plugins": [{"name": "accelerator", "path": str(REPO_ROOT)}],
            }
        )
    ]
    calls = []
    if read_rule:
        calls.append(("Read", {"file_path": str(RULE)}))
    if bash:
        calls.append(("Bash", {"command": bash}))
    lines += [
        _line(
            {
                "type": "assistant",
                "message": {
                    "content": [
                        {
                            "type": "tool_use",
                            "id": name,
                            "name": name,
                            "input": i,
                        }
                    ]
                },
            }
        )
        for name, i in calls
    ]
    if text:
        lines.append(
            _line(
                {
                    "type": "assistant",
                    "message": {"content": [{"type": "text", "text": text}]},
                }
            )
        )
    lines.append(
        _line({"type": "result", "is_error": False, "permission_denials": []})
    )
    return "\n".join(lines) + "\n"


class Scripted:
    """Answer each turn with its scripted log and exit, or a timeout."""

    def __init__(self, *turns: tuple[str, int | str]) -> None:
        self.turns = list(turns)
        self.commands: list[list[str]] = []

    def __call__(self, command, root, log, env) -> int:
        self.commands.append(command)
        text, outcome = self.turns[len(self.commands) - 1]
        log.write_text(text)
        log.with_suffix(".err").write_text("boom\n")
        if outcome == "timeout":
            raise subprocess.TimeoutExpired(command, 1)
        return int(outcome)


def _plan(tmp_path: Path, name: str) -> runs.SessionPlan:
    return next(
        plan
        for plan in runs.plan_sessions(FILES, tmp_path)
        if plan.scenario.name == name
    )


def _session(tmp_path: Path, name: str, runner: Scripted) -> runs.SessionPlan:
    plan = _plan(tmp_path, name)
    runs.run_planned(
        plan,
        plugin_root=REPO_ROOT,
        evals=EVALS,
        flags=[],
        env={},
        run=runner,
    )
    return plan


def _judge(plan: runs.SessionPlan) -> runs.SessionResult:
    return runs.judge(plan, REPO_ROOT, validate=lambda path: None)


def test_the_full_run_plans_every_session_in_its_own_directory(
    tmp_path: Path,
) -> None:
    plans = runs.plan_sessions(FILES, tmp_path)

    assert len(plans) == 111
    assert len({plan.directory for plan in plans}) == 111
    assert _plan(tmp_path, "a1-c2-partial").directory == (
        tmp_path / "A1" / "a1-c2-partial" / "run-1"
    )


def test_family_verb_and_scenario_filters_combine(tmp_path: Path) -> None:
    by_family = runs.plan_sessions(FILES, tmp_path, families=("G1", "S1"))
    by_verb = runs.plan_sessions(
        FILES, tmp_path, families=("G1", "S1"), verbs=("ask",)
    )
    by_name = runs.plan_sessions(
        FILES, tmp_path, verbs=("ask",), scenarios=("s1-complete-ask",)
    )

    assert len(by_family) == 12
    assert {p.scenario.verb for p in by_verb} == {"ask"}
    assert len(by_verb) == 6
    assert [p.directory.name for p in by_name] == ["run-1", "run-2", "run-3"]


def test_a_re_check_judges_only_sessions_on_disk(tmp_path: Path) -> None:
    plans = runs.plan_sessions(FILES, tmp_path, families=("G1",))
    plans[2].directory.mkdir(parents=True)

    assert runs.sessions_on_disk(plans) == [plans[2]]


def test_a_session_meeting_its_criteria_passes(tmp_path: Path) -> None:
    plan = _session(
        tmp_path, "g1-briefed-ask", Scripted((_log(text=BRIEFED), 0))
    )

    result = _judge(plan)

    assert result.status == "passed", result.failures


def test_a_wrong_refusal_fails_with_a_located_line(tmp_path: Path) -> None:
    plan = _session(tmp_path, "g1-briefed-ask", Scripted((_log(text="No."), 0)))

    result = _judge(plan)

    assert result.status == "failed"
    log = plan.directory / "turn-1.jsonl"
    assert result.failures[0].startswith(f"{plan.directory} turn 1 refusal: ")
    assert result.failures[0].endswith(f"({log})")


def test_an_errored_turn_skips_its_criteria(tmp_path: Path) -> None:
    plan = _session(tmp_path, "g1-briefed-ask", Scripted((_log(text="No."), 1)))

    result = _judge(plan)

    assert result.status == "errored"
    assert result.failures == []
    assert result.errored == [
        runs.ErroredTurn(1, "exit code 1", plan.directory / "turn-1.err")
    ]


def test_a_resumed_turn_exiting_with_an_empty_log_errors_the_session(
    tmp_path: Path,
) -> None:
    plan = _session(
        tmp_path,
        "r2-decline",
        Scripted((_log(text=ANSWERED, read_rule=True), 0), ("", 1)),
    )

    assert _judge(plan).status == "errored"


def test_a_criterion_broken_before_a_timeout_stays_failed(
    tmp_path: Path,
) -> None:
    plan = _session(
        tmp_path,
        "r2-decline",
        Scripted((_log(text="No answer."), 0), ("", "timeout")),
    )

    result = _judge(plan)

    assert result.status == "failed"
    assert result.errored == [
        runs.ErroredTurn(2, "timeout", plan.directory / "turn-2.err")
    ]


def test_a_broken_turn_check_stops_the_session_and_fails_it(
    tmp_path: Path,
) -> None:
    runner = Scripted((_log(text=ANSWERED, read_rule=True, bash="ls"), 0))
    plan = _session(tmp_path, "r2-decline", runner)

    result = _judge(plan)

    assert len(runner.commands) == 1
    assert (plan.directory / "turn-2.exit").read_text() == "not-run"
    assert result.status == "failed"
    assert any("allow-list" in failure for failure in result.failures)


def test_a_session_that_never_started_is_errored(tmp_path: Path) -> None:
    plan = _plan(tmp_path, "g1-briefed-ask")
    plan.directory.mkdir(parents=True)

    assert _judge(plan).status == "errored"


def _write_outcomes(tmp_path: Path) -> dict[str, runs.SessionPlan]:
    return {
        "passed": _session(
            tmp_path, "g1-briefed-ask", Scripted((_log(text=BRIEFED), 0))
        ),
        "failed": _session(
            tmp_path, "g1-briefed-report", Scripted((_log(text="No."), 0))
        ),
        "errored": _session(
            tmp_path, "g1-outlined-ask", Scripted((_log(text="No."), 1))
        ),
    }


def test_rerunning_errored_sessions_moves_only_those_aside(
    tmp_path: Path,
) -> None:
    sessions = _write_outcomes(tmp_path)
    on_disk = runs.sessions_on_disk(runs.plan_sessions(FILES, tmp_path))

    replanned = runs.set_aside_errored(on_disk, REPO_ROOT, lambda path: None)

    errored = sessions["errored"].directory
    assert replanned == [sessions["errored"]]
    assert not errored.exists()
    assert (errored.parent / "run-1.errored-1" / "turn-1.exit").exists()
    for kept in ("passed", "failed"):
        assert (sessions[kept].directory / "turn-1.exit").exists()


def test_a_second_set_aside_takes_the_next_free_suffix(tmp_path: Path) -> None:
    errored = _write_outcomes(tmp_path)["errored"].directory
    (errored.parent / "run-1.errored-1").mkdir()

    runs.set_aside_errored(
        runs.sessions_on_disk(runs.plan_sessions(FILES, tmp_path)),
        REPO_ROOT,
        lambda path: None,
    )

    assert (errored.parent / "run-1.errored-2").is_dir()


def test_results_record_the_run_s_settings(tmp_path: Path) -> None:
    settings = runs.RunSettings("2.1.300", "claude-test", "/config")
    runs.save_results([], tmp_path, settings)

    assert runs.load_settings(tmp_path) == settings


@pytest.mark.parametrize(
    ("model", "config_dir"),
    [("other", ""), ("", "/elsewhere")],
)
def test_a_rerun_refuses_settings_that_differ_from_the_run_s(
    model: str, config_dir: str
) -> None:
    recorded = runs.RunSettings("2.1.300", "claude-test", "/config")

    assert runs.settings_conflict(recorded, model, config_dir)


def test_a_rerun_takes_the_run_s_settings_when_none_are_given() -> None:
    recorded = runs.RunSettings("2.1.300", "claude-test", "/config")

    assert runs.settings_conflict(recorded, "", "") is None
    assert runs.settings_conflict(recorded, "claude-test", "/config") is None


def test_the_summary_ends_with_each_errored_turn_and_the_rerun_command(
    tmp_path: Path,
) -> None:
    sessions = _write_outcomes(tmp_path)
    plans = list(sessions.values())
    results = [_judge(plan) for plan in plans]
    settings = runs.RunSettings("2.1.300", "claude-test", "/config")

    lines = runs.summary(plans, results, tmp_path, settings).splitlines()

    errored = sessions["errored"].directory
    assert "| G1 | 1/3 | 1 | 1 |" in lines
    assert lines[-2:] == [
        f"{errored} turn 1 errored: exit code 1 ({errored / 'turn-1.err'})",
        "mise run eval:ask-and-report --rerun-errored "
        f"--out {tmp_path} --model claude-test --config-dir /config",
    ]


def test_a_summary_with_no_errored_session_names_no_rerun(
    tmp_path: Path,
) -> None:
    plan = _session(
        tmp_path, "g1-briefed-ask", Scripted((_log(text=BRIEFED), 0))
    )
    settings = runs.RunSettings("2.1.300", "", "")

    text = runs.summary([plan], [_judge(plan)], tmp_path, settings)

    assert "--rerun-errored" not in text


def test_a_result_records_the_model_each_session_ran(tmp_path: Path) -> None:
    plan = _session(
        tmp_path, "g1-briefed-ask", Scripted((_log(text=BRIEFED), 0))
    )

    assert _judge(plan).models == ["claude-test"]
