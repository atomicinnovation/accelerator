"""Running the ask-and-report eval end to end: gate, plan, run and judge."""

import tempfile
from pathlib import Path

from invoke.exceptions import Exit

from tasks.eval.ask_and_report import runs as ask_and_report_runs
from tasks.eval.ask_and_report.conversation import session_env, turn_flags
from tasks.eval.ask_and_report.layout import REFERENCE_CORPUS
from tasks.eval.ask_and_report.preflight import preflight_problems
from tasks.eval.ask_and_report.scenarios import load_scenario_files
from tasks.eval.ask_and_report.startup import (
    authentication_refusal,
    claude_version,
    version_refusal,
)
from tasks.eval.toolchain import (
    DEV_LAUNCHER_REMINDER,
    Toolchain,
    require_dev_launcher_marker,
)


def _names(option: str) -> tuple[str, ...]:
    return tuple(name for name in option.split(",") if name)


def _claude_version_allowed(root: Path) -> str:
    require_dev_launcher_marker(root)
    version = claude_version()
    if refusal := version_refusal(version):
        raise Exit(refusal, code=1)
    return version


def _run_settings(
    directory: Path,
    given: ask_and_report_runs.RunSettings,
    check_only: bool,
    rerun_errored: bool,
) -> ask_and_report_runs.RunSettings:
    recorded_run = (directory / ask_and_report_runs.RESULTS).exists()
    if rerun_errored and not recorded_run:
        raise Exit(
            f"{directory} holds no {ask_and_report_runs.RESULTS}, so its run "
            "never finished; start it again without --rerun-errored",
            code=1,
        )
    if check_only and not recorded_run:
        return ask_and_report_runs.RunSettings("", "", "")
    if not (rerun_errored or check_only):
        return given
    recorded = ask_and_report_runs.load_settings(directory)
    if rerun_errored and (
        conflict := ask_and_report_runs.settings_conflict(recorded, given)
    ):
        raise Exit(conflict, code=1)
    return recorded


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
    errored = ask_and_report_runs.errored_sessions(plans, root, toolchain)
    print(f"Rerunning {len(errored)} errored sessions")
    return errored


def _print_failures(results: list[ask_and_report_runs.SessionResult]) -> None:
    for result in results:
        if result.failures:
            print(f"\n{result.session}")
            for failure in result.failures:
                print(f"  {failure}")


def run_ask_and_report(
    root: Path,
    out: str,
    family: str,
    verb: str,
    scenario: str,
    jobs: int,
    model: str,
    config_dir: str,
    check_only: bool,
    rerun_errored: bool,
) -> None:
    if rerun_errored and not out:
        raise Exit("--rerun-errored needs --out, the run to rerun", code=1)
    if check_only and not out:
        raise Exit("--check-only needs --out, the run to judge", code=1)
    evals = root / REFERENCE_CORPUS
    files = load_scenario_files(evals / "scenarios")
    if unmatched := ask_and_report_runs.unmatched_filters(
        files, _names(family), _names(verb), _names(scenario)
    ):
        raise Exit("\n".join(unmatched), code=1)
    version = "" if check_only else _claude_version_allowed(root)
    toolchain = Toolchain.debug(root)
    if problems := preflight_problems(files, toolchain):
        raise Exit("\n".join(problems), code=1)
    directory = Path(out) if out else Path(tempfile.mkdtemp(prefix="ask-"))
    settings = _run_settings(
        directory,
        ask_and_report_runs.RunSettings(version, model, config_dir),
        check_only,
        rerun_errored,
    )
    flags = turn_flags(root, settings.model or None)
    env = session_env(
        toolchain, Path(settings.config_dir) if settings.config_dir else None
    )
    try:
        if not check_only and (
            refusal := authentication_refusal(root, flags, env)
        ):
            raise Exit(refusal, code=1)
        plans = ask_and_report_runs.plan_sessions(
            files, directory, _names(family), _names(verb), _names(scenario)
        )
        if check_only or rerun_errored:
            plans = ask_and_report_runs.sessions_on_disk(plans)
        if not plans:
            raise Exit(f"no session to judge under {directory}", code=1)
        to_run = _sessions_to_run(
            plans, root, toolchain, check_only, rerun_errored
        )
        print(f"Sessions and results: {directory}")
        print(f"Planned sessions: {len(to_run)}")
        ask_and_report_runs.run_eval(
            to_run, root, flags, env, jobs, replacing=rerun_errored
        )
        results = ask_and_report_runs.judge_all(plans, root, toolchain)
        ask_and_report_runs.save_results(results, directory, settings)
        print(ask_and_report_runs.summary(plans, results, directory, settings))
        _print_failures(results)
        if any(result.status != "passed" for result in results):
            raise Exit(code=1)
    finally:
        if not check_only:
            print(DEV_LAUNCHER_REMINDER)
