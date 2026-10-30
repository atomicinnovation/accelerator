import tempfile
from pathlib import Path

from invoke import Context, task
from invoke.exceptions import Exit

from tasks.eval.citing_once.fixtures import load_fixtures
from tasks.eval.citing_once.rule import key_order
from tasks.eval.citing_once.runs import (
    fixture_problems,
    plan_sessions,
    run_eval,
    save_results,
    sessions_on_disk,
    summary,
)
from tasks.eval.citing_once.sessions import RULE
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
    ``PATH``, network access and model usage.
    """
    root = repo_root()
    fixtures = load_fixtures(root / FIXTURES)
    problems = fixture_problems(fixtures, key_order((root / RULE).read_text()))
    if problems:
        raise Exit("\n".join(problems), code=1)
    directory = Path(out) if out else Path(tempfile.mkdtemp(prefix="cite-"))
    families = tuple(name for name in family.split(",") if name)
    plans = plan_sessions(fixtures, directory, families)
    if check_only:
        plans = sessions_on_disk(plans)
    results = run_eval(plans, root, root / SEED, jobs, check_only)
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
