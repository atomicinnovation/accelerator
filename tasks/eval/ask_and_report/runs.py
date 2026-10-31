"""Plan, run and judge the ask-and-report eval's sessions.

A session fails when it breaks a criterion or a check, whatever else its
turns did. Otherwise it is errored when any turn failed for a reason the
model did not cause, which a rerun may clear; a failed session is never
rerun, so a 2/3 judgement can never pass as 3/3.
"""

import json
import subprocess
import uuid
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Literal

from tasks.eval.ask_and_report.checks import (
    Bounds,
    breaks_a_turn_check,
    session_check_failures,
    turn_check_failures,
)
from tasks.eval.ask_and_report.conversation import (
    NOT_RUN,
    Conversation,
    Runner,
    opening_prompt,
    run_conversation,
    turn_file,
)
from tasks.eval.ask_and_report.projects import build_project
from tasks.eval.ask_and_report.scenarios import Scenario, ScenarioFile
from tasks.eval.ask_and_report.turns import (
    TurnRecord,
    errored,
    load_snapshot,
)
from tasks.eval.ask_and_report.verdicts import CRITERIA, Judging
from tasks.eval.headless.claude import prepare_repository, run_session
from tasks.eval.headless.transcript import Event, Init, events
from tasks.eval.toolchain import Toolchain

RESULTS = "results.json"
PROJECT = "project"

type Status = Literal["passed", "failed", "errored"]
type Validator = Callable[[Path], str | None]


@dataclass(frozen=True)
class SessionPlan:
    family: str
    scenario: Scenario
    run: int
    directory: Path

    @property
    def project(self) -> Path:
        return self.directory / PROJECT


def plan_sessions(
    files: Sequence[ScenarioFile],
    out: Path,
    families: Sequence[str] = (),
    verbs: Sequence[str] = (),
    scenarios: Sequence[str] = (),
) -> list[SessionPlan]:
    """Plan each scenario's runs, narrowed by every filter given."""
    return [
        SessionPlan(
            family=file.family,
            scenario=scenario,
            run=run,
            directory=out / file.family / scenario.name / f"run-{run}",
        )
        for file in files
        if not families or file.family in families
        for scenario in file.scenarios
        if not verbs or scenario.verb in verbs
        if not scenarios or scenario.name in scenarios
        for run in range(1, scenario.times + 1)
    ]


def sessions_on_disk(plans: Sequence[SessionPlan]) -> list[SessionPlan]:
    return [plan for plan in plans if plan.directory.exists()]


def run_planned(
    plan: SessionPlan,
    plugin_root: Path,
    evals: Path,
    flags: Sequence[str],
    env: dict[str, str],
    run: Runner = run_session,
) -> None:
    plan.directory.mkdir(parents=True, exist_ok=False)
    build_project(plan.scenario.setup, plan.project, evals)
    prepare_repository(plan.project)
    bounds = Bounds(plan.project, plugin_root)
    turns = plan.scenario.turns
    run_conversation(
        Conversation(
            prompt=opening_prompt(plan.scenario.verb, plan.scenario.question),
            replies=tuple(turn.reply or "" for turn in turns[1:]),
            writes=tuple(turn.writes for turn in turns),
            session_id=str(uuid.uuid4()),
            root=plan.project,
            directory=plan.directory,
        ),
        flags=flags,
        env=env,
        stops_after=lambda record: breaks_a_turn_check(record, bounds),
        run=run,
    )


def load_records(plan: SessionPlan) -> list[TurnRecord]:
    """Read back each turn that ran, as the turn loop recorded it."""
    initial_file = turn_file(plan.directory, 0, ".json")
    if not initial_file.exists():
        return []
    initial = load_snapshot(initial_file)
    records: list[TurnRecord] = []
    earlier: list[tuple[Event, ...]] = []
    before = initial
    for number, turn in enumerate(plan.scenario.turns, start=1):
        exit_file = turn_file(plan.directory, number, ".exit")
        status = exit_file.read_text().strip() if exit_file.exists() else None
        if status == NOT_RUN:
            break
        log = turn_file(plan.directory, number, ".jsonl")
        snapshot_file = turn_file(plan.directory, number, ".json")
        after = (
            load_snapshot(snapshot_file) if snapshot_file.exists() else before
        )
        record = TurnRecord(
            number=number,
            reply=turn.reply,
            writes=turn.writes,
            events=tuple(events(log.read_text())) if log.exists() else (),
            logged=log.exists(),
            exit_status=status,
            before=before,
            after=after,
            initial=initial,
            earlier=tuple(earlier),
        )
        records.append(record)
        earlier.append(record.events)
        before = after
    return records


@dataclass(frozen=True)
class ErroredTurn:
    turn: int
    reason: str
    stderr: Path


@dataclass
class SessionResult:
    session: str
    family: str
    scenario: str
    failures: list[str] = field(default_factory=list)
    errored: list[ErroredTurn] = field(default_factory=list)
    models: list[str] = field(default_factory=list)

    @property
    def status(self) -> Status:
        if self.failures:
            return "failed"
        return "errored" if self.errored else "passed"


_KEYS = {kind: key for key, kind in CRITERIA.items()}


def judge(
    plan: SessionPlan, plugin_root: Path, validate: Validator
) -> SessionResult:
    result = SessionResult(
        session=str(plan.directory),
        family=plan.family,
        scenario=plan.scenario.name,
    )
    records = load_records(plan)
    if not records:
        result.errored.append(
            ErroredTurn(1, "no log", turn_file(plan.directory, 1, ".err"))
        )
        return result
    bounds = Bounds(plan.project, plugin_root)
    judging = Judging(plan.project, plan.scenario.verb, validate)

    def located(record: TurnRecord, what: str, problem: str) -> str:
        log = turn_file(plan.directory, record.number, ".jsonl")
        return (
            f"{plan.directory} turn {record.number} {what}: {problem} ({log})"
        )

    for record, turn in zip(records, plan.scenario.turns, strict=False):
        result.failures += [
            located(record, "check", problem)
            for problem in turn_check_failures(record, bounds)
        ]
        result.models += [e.model for e in record.events if isinstance(e, Init)]
        if reason := errored(record):
            stderr = turn_file(plan.directory, record.number, ".err")
            result.errored.append(ErroredTurn(record.number, reason, stderr))
            continue
        result.failures += [
            located(record, _KEYS[type(criterion)], problem)
            for criterion in turn.expect
            for problem in criterion.judge(record, judging)
        ]
    result.failures += [
        located(records[-1], "check", problem)
        for problem in session_check_failures(records, bounds)
    ]
    result.models = list(dict.fromkeys(result.models))
    return result


def corpus_validator(toolchain: Toolchain, root: Path) -> Validator:
    def validate(path: Path) -> str | None:
        validated = subprocess.run(
            [str(toolchain.corpus), "frontmatter", "validate", "--file", path],
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if validated.returncode == 0:
            return None
        return (validated.stdout + validated.stderr).strip()

    return validate


def judge_all(
    plans: Sequence[SessionPlan], plugin_root: Path, toolchain: Toolchain
) -> list[SessionResult]:
    return [
        judge(plan, plugin_root, corpus_validator(toolchain, plan.project))
        for plan in plans
    ]


def run_eval(
    plans: Sequence[SessionPlan],
    plugin_root: Path,
    evals: Path,
    flags: Sequence[str],
    env: dict[str, str],
    jobs: int,
) -> None:
    """Run every planned session, then raise the first harness error, if any.

    ``Executor.map`` would cancel every queued session at the first error.
    """
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = [
            pool.submit(run_planned, plan, plugin_root, evals, flags, env)
            for plan in plans
        ]
    for future in futures:
        future.result()


def _set_aside(directory: Path) -> None:
    attempt = 1
    while (
        aside := directory.with_name(f"{directory.name}.errored-{attempt}")
    ).exists():
        attempt += 1
    directory.rename(aside)


def set_aside_errored(
    plans: Sequence[SessionPlan], plugin_root: Path, toolchain: Toolchain
) -> list[SessionPlan]:
    """Move each errored session aside and return its plan to run again.

    A session is re-judged from disk first, so an edited exit status counts.
    """
    replanned = [
        plan
        for plan, result in zip(
            plans, judge_all(plans, plugin_root, toolchain), strict=True
        )
        if result.status == "errored"
    ]
    for plan in replanned:
        _set_aside(plan.directory)
    return replanned


@dataclass(frozen=True)
class RunSettings:
    claude_version: str
    model: str
    config_dir: str


def settings_conflict(
    recorded: RunSettings, model: str, config_dir: str
) -> str | None:
    """Name a rerun option that differs from the run it reruns, if any."""
    for option, given, held in (
        ("--model", model, recorded.model),
        ("--config-dir", config_dir, recorded.config_dir),
    ):
        if given and given != held:
            return f"{option} {given} differs from the run's {held or 'none'}"
    return None


def save_results(
    results: Sequence[SessionResult], out: Path, settings: RunSettings
) -> None:
    (out / RESULTS).write_text(
        json.dumps(
            asdict(settings)
            | {
                "sessions": [
                    asdict(result)
                    | {
                        "status": result.status,
                        "errored": [
                            {"turn": e.turn, "reason": e.reason}
                            for e in result.errored
                        ],
                    }
                    for result in results
                ]
            },
            indent=2,
        )
    )


def load_settings(out: Path) -> RunSettings:
    recorded = json.loads((out / RESULTS).read_text())
    return RunSettings(
        claude_version=recorded["claude_version"],
        model=recorded["model"],
        config_dir=recorded["config_dir"],
    )


def rerun_command(out: Path, settings: RunSettings) -> str:
    command = f"mise run eval:ask-and-report --rerun-errored --out {out}"
    if settings.model:
        command += f" --model {settings.model}"
    if settings.config_dir:
        command += f" --config-dir {settings.config_dir}"
    return command


def summary(
    plans: Sequence[SessionPlan],
    results: Sequence[SessionResult],
    out: Path,
    settings: RunSettings,
) -> str:
    rows = ["| Family | Passed | Failed | Errored |", "|---|---|---|---|"]
    for family in dict.fromkeys(plan.family for plan in plans):
        mine = [r for r in results if r.family == family]
        counts = {
            status: sum(r.status == status for r in mine)
            for status in ("passed", "failed", "errored")
        }
        rows.append(
            f"| {family} | {counts['passed']}/{len(mine)} | "
            f"{counts['failed']} | {counts['errored']} |"
        )
    errored = [r for r in results if r.status == "errored"]
    rows += [
        f"{r.session} turn {turn.turn} errored: {turn.reason} ({turn.stderr})"
        for r in errored
        for turn in r.errored
    ]
    if errored:
        rows.append(rerun_command(out, settings))
    return "\n".join(rows)
