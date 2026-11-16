import tempfile
from pathlib import Path

from invoke import Context, task
from invoke.exceptions import Exit

from tasks.eval.ask_and_report.orchestration import run_ask_and_report
from tasks.eval.citing_once.fixtures import load_fixtures
from tasks.eval.citing_once.runs import (
    fixture_problems,
    plan_sessions,
    run_eval,
    save_results,
    sessions_on_disk,
    summary,
)
from tasks.eval.documents.rule import RULE, key_order
from tasks.eval.toolchain import (
    DEV_LAUNCHER_REMINDER,
    require_dev_launcher_marker,
)
from tasks.shared.sources import repo_root

FIXTURES = "skills/research/outputters/citing-once/evals"
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
    working tree's debug CLI builds, which needs the dev-launcher marker
    unless ``--check-only`` only judges sessions already run.
    """
    root = repo_root()
    if not check_only:
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
    run_ask_and_report(
        repo_root(),
        out=out,
        family=family,
        verb=verb,
        scenario=scenario,
        jobs=jobs,
        model=model,
        config_dir=config_dir,
        check_only=check_only,
        rerun_errored=rerun_errored,
    )
