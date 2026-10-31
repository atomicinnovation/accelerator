import tempfile
from pathlib import Path

from invoke import Context, task
from invoke.exceptions import Exit

from tasks.eval.ask_and_report import runs as ask_and_report_runs
from tasks.eval.ask_and_report.conversation import session_env, turn_flags
from tasks.eval.ask_and_report.preflight import preflight_problems
from tasks.eval.ask_and_report.scenarios import load_scenario_files
from tasks.eval.ask_and_report.startup import (
    authentication_refusal,
    claude_version,
    version_refusal,
)
from tasks.eval.citing_once.fixtures import load_fixtures
from tasks.eval.citing_once.runs import (
    fixture_problems,
    plan_sessions,
    run_eval,
    save_results,
    sessions_on_disk,
    summary,
)
from tasks.eval.citing_once.sessions import RULE
from tasks.eval.documents.rule import key_order
from tasks.eval.toolchain import (
    DEV_LAUNCHER_REMINDER,
    Toolchain,
    require_dev_launcher_marker,
)
from tasks.shared.sources import repo_root

FIXTURES = "skills/research/outputters/citing-once/evals"
REFERENCE_CORPUS = "skills/research/research-topic/evals"
SEED = "cli/corpus-cli/tests/fixtures/topic-research-deepened-set"


@task(
    name="citing-once",
    help={
        "out": "Directory for projects, logs and results (default: a new "
        "temporary directory)",
        "family": "Comma-separated fixture families to run, e.g. C1,S2",
        "jobs": "Sessions to run at once",
        "check_only": "Judge the sessions already in --out without running any",
    },
)
def citing_once(
    context: Context,
    out: str = "",
    family: str = "",
    jobs: int = 6,
    check_only: bool = False,
) -> None:
    """Run the citing-once behavioural eval against this repository's plugin.

    Each session is a headless ``claude -p`` run, so this needs ``claude`` on
    ``PATH``, network access and model usage. Sessions and the judge run the
    working tree's debug CLI builds, which needs the dev-launcher marker.
    """
    root = repo_root()
    require_dev_launcher_marker(root)
    fixtures = load_fixtures(root / FIXTURES)
    problems = fixture_problems(fixtures, key_order((root / RULE).read_text()))
    if problems:
        raise Exit("\n".join(problems), code=1)
    directory = Path(out) if out else Path(tempfile.mkdtemp(prefix="cite-"))
    families = tuple(name for name in family.split(",") if name)
    plans = plan_sessions(fixtures, directory, families)
    if check_only:
        plans = sessions_on_disk(plans)
    try:
        results = run_eval(plans, root, root / SEED, jobs, check_only)
    finally:
        if not check_only:
            print(DEV_LAUNCHER_REMINDER)
    save_results(results, directory)
    print(f"Sessions and results: {directory}")
    print(summary(plans, results))
    failed = [result for result in results if not result.passed]
    for result in failed:
        print(f"\n{result.session}")
        for problem in (
            result.criterion_failures
            + result.entry_differences
            + result.log_failures
            + [result.validation_failure or ""]
        ):
            if problem:
                print(f"  {problem}")
    if failed:
        raise Exit(code=1)


def _names(option: str) -> tuple[str, ...]:
    return tuple(name for name in option.split(",") if name)


def _claude_version_allowed(root: Path) -> str:
    require_dev_launcher_marker(root)
    version = claude_version()
    if refusal := version_refusal(version):
        raise Exit(refusal, code=1)
    return version


def _recorded_settings(
    directory: Path, model: str, config_dir: str
) -> tuple[str, str]:
    recorded = ask_and_report_runs.load_settings(directory)
    if conflict := ask_and_report_runs.settings_conflict(
        recorded, model, config_dir
    ):
        raise Exit(conflict, code=1)
    return recorded.model, recorded.config_dir


def _sessions_to_run(
    plans: list[ask_and_report_runs.SessionPlan],
    root: Path,
    toolchain: Toolchain,
    check_only: bool,
    rerun_errored: bool,
) -> list[ask_and_report_runs.SessionPlan]:
    if check_only:
        return []
    if not rerun_errored:
        return plans
    errored = ask_and_report_runs.set_aside_errored(plans, root, toolchain)
    print(f"Rerunning {len(errored)} errored sessions")
    return errored


def _print_failures(results: list[ask_and_report_runs.SessionResult]) -> None:
    for result in results:
        if result.failures:
            print(f"\n{result.session}")
            for failure in result.failures:
                print(f"  {failure}")


@task(
    name="ask-and-report",
    help={
        "out": "Directory for projects, logs and results (default: a new "
        "temporary directory)",
        "family": "Comma-separated scenario families to run, e.g. G1,A2",
        "verb": "Comma-separated verbs to run: ask, report",
        "scenario": "Comma-separated scenario names to run",
        "jobs": "Sessions to run at once",
        "model": "Model every turn runs on (default: claude's own)",
        "config_dir": "CLAUDE_CONFIG_DIR for every session (default: "
        "inherited); it must hold its own login",
        "check_only": "Judge the sessions already in --out without running any",
        "rerun_errored": "Rerun only the errored sessions in --out, with "
        "the run's model and config dir",
    },
)
def ask_and_report(
    context: Context,
    out: str = "",
    family: str = "",
    verb: str = "",
    scenario: str = "",
    jobs: int = 6,
    model: str = "",
    config_dir: str = "",
    check_only: bool = False,
    rerun_errored: bool = False,
) -> None:
    """Run the ask-and-report behavioural eval against this repository.

    Each session is a multi-turn headless ``claude -p`` conversation over
    the reference corpus, so this needs ``claude`` on ``PATH``, network
    access and model usage. Sessions and the judge run the working tree's
    debug CLI builds, which needs the dev-launcher marker. A failed session
    is never rerun: only an errored one, whose failure the model did not
    cause, is.
    """
    if rerun_errored and not out:
        raise Exit("--rerun-errored needs --out, the run to rerun", code=1)
    root = repo_root()
    evals = root / REFERENCE_CORPUS
    version = "" if check_only else _claude_version_allowed(root)
    toolchain = Toolchain.debug(root)
    files = load_scenario_files(evals / "scenarios")
    if problems := preflight_problems(files, evals, toolchain):
        raise Exit("\n".join(problems), code=1)
    directory = Path(out) if out else Path(tempfile.mkdtemp(prefix="ask-"))
    if rerun_errored:
        model, config_dir = _recorded_settings(directory, model, config_dir)
    settings = ask_and_report_runs.RunSettings(version, model, config_dir)
    flags = turn_flags(root, model or None)
    env = session_env(toolchain, Path(config_dir) if config_dir else None)
    try:
        if not check_only and (
            refusal := authentication_refusal(root, evals, flags, env)
        ):
            raise Exit(refusal, code=1)
        plans = ask_and_report_runs.plan_sessions(
            files, directory, _names(family), _names(verb), _names(scenario)
        )
        if check_only or rerun_errored:
            plans = ask_and_report_runs.sessions_on_disk(plans)
        to_run = _sessions_to_run(
            plans, root, toolchain, check_only, rerun_errored
        )
        print(f"Sessions and results: {directory}")
        print(f"Planned sessions: {len(to_run)}")
        ask_and_report_runs.run_eval(to_run, root, evals, flags, env, jobs)
        results = ask_and_report_runs.judge_all(plans, root, toolchain)
        ask_and_report_runs.save_results(results, directory, settings)
        print(ask_and_report_runs.summary(plans, results, directory, settings))
        _print_failures(results)
        if any(result.status != "passed" for result in results):
            raise Exit(code=1)
    finally:
        if not check_only:
            print(DEV_LAUNCHER_REMINDER)
