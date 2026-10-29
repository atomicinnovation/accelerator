"""Plan, run and judge the citing-once eval's sessions."""

import json
import subprocess
from collections.abc import Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass, field
from pathlib import Path

from tasks.eval.citing_once.fixtures import Composition, Fixture
from tasks.eval.citing_once.logs import (
    composer_log_failures,
    synthesise_log_failures,
    tool_calls,
)
from tasks.eval.citing_once.projects import Project, build_project
from tasks.eval.citing_once.rule import key_order
from tasks.eval.citing_once.sessions import (
    RULE,
    composer_prompt,
    prepare_repository,
    run_session,
    session_command,
    synthesise_prompt,
)
from tasks.eval.citing_once.sources import parse_sources
from tasks.eval.citing_once.verdicts import (
    criterion_failures,
    entry_differences,
    isolation_conflicts,
    predict,
)

LOG = "session.jsonl"
EXIT_CODE = "exit-code"
LAYOUT = "project.json"


@dataclass(frozen=True)
class SessionPlan:
    family: str
    composition: Composition
    order: tuple[str, ...]
    directory: Path


@dataclass
class SessionResult:
    session: str
    exit_code: int | None
    criteria: int
    criterion_failures: list[str] = field(default_factory=list)
    entry_differences: list[str] = field(default_factory=list)
    log_failures: list[str] = field(default_factory=list)
    validation_failure: str | None = None

    @property
    def passed(self) -> bool:
        return (
            self.exit_code == 0
            and not self.criterion_failures
            and not self.entry_differences
            and not self.log_failures
            and self.validation_failure is None
        )


def plan_sessions(
    fixtures: Sequence[Fixture], out: Path, families: Sequence[str]
) -> list[SessionPlan]:
    return [
        SessionPlan(
            family=fixture.family,
            composition=composition,
            order=order,
            directory=out
            / fixture.family
            / composition.name
            / f"order-{order_number}-run-{run}",
        )
        for fixture in fixtures
        if not families or fixture.family in families
        for composition in fixture.compositions
        for order_number, order in enumerate(composition.orders, start=1)
        for run in range(1, composition.times + 1)
    ]


def sessions_on_disk(plans: Sequence[SessionPlan]) -> list[SessionPlan]:
    return [plan for plan in plans if (plan.directory / LAYOUT).exists()]


def fixture_problems(
    fixtures: Sequence[Fixture], recommended: Sequence[str]
) -> list[str]:
    problems: list[str] = []
    for fixture in fixtures:
        for composition in fixture.compositions:
            problems += [
                f"{composition.name}: {conflict}"
                for conflict in isolation_conflicts(composition)
            ]
            for order in composition.orders:
                problems += [
                    f"{composition.name}: the rule disagrees with {failure}"
                    for failure in criterion_failures(
                        composition, predict(composition, order, recommended)
                    )
                ]
    return problems


def _save_layout(project: Project, directory: Path) -> None:
    (directory / LAYOUT).write_text(
        json.dumps(
            {
                "root": str(project.root),
                "set_root": str(project.set_root),
                "slug": project.slug,
                "inputs": [str(path) for path in project.inputs],
                "output": str(project.output),
            },
            indent=2,
        )
    )


def _load_layout(directory: Path) -> Project:
    data = json.loads((directory / LAYOUT).read_text())
    return Project(
        root=Path(data["root"]),
        set_root=Path(data["set_root"]),
        slug=data["slug"],
        inputs=tuple(Path(path) for path in data["inputs"]),
        output=Path(data["output"]),
    )


def run_planned(plan: SessionPlan, plugin_root: Path, seed: Path) -> None:
    plan.directory.mkdir(parents=True, exist_ok=False)
    project = build_project(
        plan.composition, plan.order, plan.directory / "project", seed
    )
    _save_layout(project, plan.directory)
    prepare_repository(project)
    prompt = (
        composer_prompt(project, plugin_root)
        if plan.composition.carrier == "composer"
        else synthesise_prompt(project)
    )
    code = run_session(
        session_command(prompt, plugin_root), project, plan.directory / LOG
    )
    (plan.directory / EXIT_CODE).write_text(f"{code}\n")


def _exit_code(directory: Path) -> int | None:
    recorded = directory / EXIT_CODE
    return int(recorded.read_text()) if recorded.exists() else None


def _validation_failure(project: Project) -> str | None:
    validated = subprocess.run(
        [
            "accelerator",
            "corpus",
            "frontmatter",
            "validate",
            "--file",
            str(project.output),
        ],
        cwd=project.root,
        capture_output=True,
        text=True,
        check=False,
    )
    if validated.returncode == 0:
        return None
    return (validated.stdout + validated.stderr).strip()


def judge(
    plan: SessionPlan, plugin_root: Path, recommended: Sequence[str]
) -> SessionResult:
    project = _load_layout(plan.directory)
    result = SessionResult(
        session=str(plan.directory),
        exit_code=_exit_code(plan.directory),
        criteria=len(plan.composition.criteria),
    )
    if not project.output.exists():
        result.criterion_failures = [
            f"{c.id}: no output written" for c in plan.composition.criteria
        ]
        return result
    written = parse_sources(project.output.read_text())
    result.criterion_failures = criterion_failures(plan.composition, written)
    result.entry_differences = entry_differences(
        predict(plan.composition, plan.order, recommended), written
    )
    calls = tool_calls((plan.directory / LOG).read_text())
    rule = plugin_root / RULE
    result.log_failures = (
        composer_log_failures(calls, rule)
        if plan.composition.carrier == "composer"
        else synthesise_log_failures(calls, rule)
    )
    result.validation_failure = _validation_failure(project)
    return result


def run_eval(
    plans: Sequence[SessionPlan],
    plugin_root: Path,
    seed: Path,
    jobs: int,
    check_only: bool,
) -> list[SessionResult]:
    recommended = key_order((plugin_root / RULE).read_text())
    if not check_only:
        with ThreadPoolExecutor(max_workers=jobs) as pool:
            list(pool.map(lambda p: run_planned(p, plugin_root, seed), plans))
    return [judge(plan, plugin_root, recommended) for plan in plans]


def summary(
    plans: Sequence[SessionPlan], results: Sequence[SessionResult]
) -> str:
    rows = ["| Fixture | Sessions passed | Criteria held |", "|---|---|---|"]
    families = dict.fromkeys(plan.family for plan in plans)
    for family in families:
        mine = [
            result
            for plan, result in zip(plans, results, strict=True)
            if plan.family == family
        ]
        held = sum(r.criteria - len(r.criterion_failures) for r in mine)
        total = sum(r.criteria for r in mine)
        passed = sum(r.passed for r in mine)
        rows.append(f"| {family} | {passed}/{len(mine)} | {held}/{total} |")
    return "\n".join(rows)


def save_results(results: Sequence[SessionResult], out: Path) -> None:
    (out / "results.json").write_text(
        json.dumps(
            [asdict(r) | {"passed": r.passed} for r in results], indent=2
        )
    )
