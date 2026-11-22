import os
import shlex
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import (
    CARGO_TOML,
    CLI_TARGET_DIR,
    CLI_WORKSPACE_CARGO_TOML,
)
from tasks.test.cli import _MANIFEST
from tasks.test.filters import (
    LIBTEST_INTEGRATION_FILTER_HELP,
    NEXTEST_CONTRACT_FILTER_HELP,
    NODE_TEST_RUNTIME_FILTER_HELP,
    PYTEST_FILTER_HELP,
    command_line,
    run_libtest_lane,
    run_nextest_lane,
    run_pytest_lane,
    selection_from,
)
from tasks.test.unit import (
    _bare_returns_in_tests,
    _tap_counts,
    report_filtered_design_automation_run,
)

from .helpers import accelerator_env, repo_root


@task(help={"filter_": LIBTEST_INTEGRATION_FILTER_HELP})
def visualiser(context: Context, filter_: str | None = None) -> None:
    """Integration tests for the visualiser (its `tests/*.rs` targets).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature, so the cargo invocation enables that feature to include it.
    `--test '*'` rather than `--tests`, which would run the library's unit
    tests a third time: `test:unit:visualiser` already runs them under both
    feature sets.
    """
    # The cargo tests include orchestration_lifecycle.rs, which dispatches the
    # compiled launcher, so they need it on the env (built by the build:cli:dev
    # mise dependency) rather than the signed-release bootstrap. The overlay is
    # passed to each child, not written into this process.
    run_libtest_lane(
        context,
        [
            f"cargo test --manifest-path {CARGO_TOML} --test '*' "
            f"--no-default-features --features dev-frontend"
        ],
        mise_task="test:integration:visualiser",
        filter_=filter_,
        env=accelerator_env(),
    )


@task
def visualiser_os_watcher(context: Context) -> None:
    """Run the visualiser's OS file-watcher contract test (opt-in).

    Every other watcher and SSE test feeds the watcher scripted events; this
    one waits on the OS's own notifications, which a loaded macOS fseventsd
    delays by minutes, so on macOS it stays out of `test:integration` and the
    bare default task. Run it on a quiet machine after changing the watcher's
    OS adapter or the `notify` dependency.
    """
    context.run(
        f"cargo test --manifest-path {CARGO_TOML} --lib "
        f"--no-default-features --features dev-frontend -- --include-ignored "
        f"--exact watcher::tests::"
        f"the_os_watcher_reports_a_write_under_a_registered_directory"
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def dev(context: Context, filter_: str | None = None) -> None:
    """Integration tests for the dev task (real circusd, fake processes)."""
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/dev -n auto -v",
        mise_task="test:integration:dev",
        filter_=filter_,
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def entrypoint(context: Context, filter_: str | None = None) -> None:
    """Hermetic tests for the bin/accelerator plugin entry point.

    Exercises the bootstrap end-to-end against a stubbed downloader and real
    minisign signatures verified by the real accelerator-verify shim.
    """
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/entrypoint -v",
        mise_task="test:integration:entrypoint",
        filter_=filter_,
    )


@task(name="tasks", help={"filter_": PYTEST_FILTER_HELP})
def build_system(context: Context, filter_: str | None = None) -> None:
    """Run the pytest integration tests for the invoke tasks."""
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/tasks -v",
        mise_task="test:integration:tasks",
        filter_=filter_,
    )


@task
def release_guard(context: Context) -> None:
    """Prove the test-loopback guard rejects a real stripped release binary."""
    context.run("uv run pytest tests/integration/release_guard -v")


@task
def skill_invocation(context: Context) -> None:
    """Run every SKILL.md `!`-site config command in the production shape."""
    context.run("uv run pytest tests/integration/skill-invocation -v")


@task
def deny(context: Context) -> None:
    """cargo-deny native-tls/OpenSSL ban regression (offline fixtures)."""
    context.run("uv run pytest tests/integration/deny -v")


@task(help={"filter_": PYTEST_FILTER_HELP})
def pup(context: Context, filter_: str | None = None) -> None:
    """cargo-pup architecture regression (needs the nightly lane)."""
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/pup -v",
        mise_task="test:integration:pup",
        filter_=filter_,
    )


@task(help={"filter_": NEXTEST_CONTRACT_FILTER_HELP})
def tracker_contract(context: Context, filter_: str | None = None) -> None:
    """Run the RemoteTracker contract harness (excluded from test:unit:cli)."""
    run_nextest_lane(
        context,
        f"cargo nextest run --profile contract {_MANIFEST}",
        mise_task="test:integration:tracker-contract",
        filter_=filter_,
        env={"ACCELERATOR_TRACKER_CONTRACT": "1"},
    )


@task
def conformance(context: Context) -> None:
    """Producer-conformance guard: drives the real corpus validator.

    Runs the launcher-provisioning pytest lane with ACCELERATOR_CORPUS_BIN
    overlaid (via ``accelerator_env(corpus_bin=True)``) and the launcher and
    sub-binary built by the ``build:cli:dev`` and ``build:cli:corpus:dev`` mise
    dependencies, so it dispatches
    ``accelerator corpus frontmatter validate`` to the compiled sub-binary.
    """
    context.run(
        "uv run pytest tests/integration/conformance -v",
        env=accelerator_env(corpus_bin=True),
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def evals(context: Context, filter_: str | None = None) -> None:
    """Hold the CLI output skill evals quote to what the CLI prints.

    Runs against the accelerator-work built by the build:cli:work:dev
    dependency.
    """
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/evals -v",
        mise_task="test:integration:evals",
        filter_=filter_,
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def hooks(context: Context, filter_: str | None = None) -> None:
    """Integration tests for the hooks/ subtree.

    The suite runs the launcher, accelerator-vcs and accelerator-research
    built by the build:cli:dev, build:cli:vcs:dev and build:cli:research:dev
    dependencies.
    """
    run_pytest_lane(
        context,
        "uv run pytest tests/integration/hooks -v",
        mise_task="test:integration:hooks",
        filter_=filter_,
    )


_RESEARCH_DIFFERENTIAL = "uv run pytest tests/integration/research -v"


@task(help={"filter_": PYTEST_FILTER_HELP})
def research(context: Context, filter_: str | None = None) -> None:
    """Run the research guard's lexer differential against bash and zsh.

    Runs the measured baseline, the lexer-boundary rows, and a fixed-seed
    sample of mutants through the compiled accelerator-research built by the
    build:cli:research:dev dependency, and every fetch the academic profiles
    show through the same guard.
    """
    run_pytest_lane(
        context,
        _RESEARCH_DIFFERENTIAL,
        mise_task="test:integration:research",
        filter_=filter_,
    )


@task
def arxiv_queue_stress(context: Context) -> None:
    """Run 30 arXiv fetches through the queue in real time (minutes)."""
    context.run(
        f"cargo nextest run --manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p accelerator-research --features test-loopback "
        "--run-ignored only -E 'binary(=arxiv_queue_stress)'",
        pty=True,
    )


@task(help={"filter_": PYTEST_FILTER_HELP})
def research_exhaustive(context: Context, filter_: str | None = None) -> None:
    """Run the research guard's lexer differential at every position."""
    run_pytest_lane(
        context,
        _RESEARCH_DIFFERENTIAL,
        mise_task="test:integration:research-exhaustive",
        filter_=filter_,
        env={"RESEARCH_DIFFERENTIAL_EXHAUSTIVE": "1"},
    )


# The Playwright-executor suites that need a real runtime.
#
# Discovered by name, not by a glob, so this lane's file set is stated rather
# than implied and can never overlap the unit lane's `lib/*.test.js`.
_DESIGN_AUTOMATION_RUNTIME_SUITES = (
    "test-run.js",
    "daemon-runtime.test.js",
)

_PLAYWRIGHT_DIR = "skills/design/inventory-design/scripts/playwright"

# The vendored trees the runtime suites launch: the driver tree's
# playwright-core, and the bundled browser. The launcher exports each as
# ACCELERATOR_TREE_<ARTIFACT> on a tree-consuming dispatch; the lane reuses
# them when present rather than paying a second materialisation.
_RUNTIME_TREES = ("driver", "browser")

# The launcher's default cache root is the plugin's own bin/, which the
# entrypoint suite running beside this lane asserts nothing writes into.
_RUNTIME_TREE_CACHE = CLI_TARGET_DIR / "design-automation-cache"

# Today's executed total across the two runtime suites. An at-least floor,
# mirroring the unit lane: `node --test` reports a vanished suite or an
# early-returning test body as passing, so this floor and the bare-return guard
# are what catch them.
_EXPECTED_DESIGN_AUTOMATION_RUNTIME_CASES = 38


def _tree_variable(artifact: str) -> str:
    return f"ACCELERATOR_TREE_{artifact.upper()}"


def _resolve_runtime_trees(context: Context) -> dict[str, str]:
    """Resolve the materialised runtime trees, or refuse the lane.

    Prefers the launcher-exported tree paths; failing that, materialises them
    through the `build:cli:dev` launcher's `cache ensure` and reads each tree
    path from its tab-separated success line. An unresolvable runtime is a
    visible refusal rather than a silent pass.
    """
    exported = {
        artifact: os.environ.get(_tree_variable(artifact), "")
        for artifact in _RUNTIME_TREES
    }
    if all(path and Path(path).is_dir() for path in exported.values()):
        return {
            _tree_variable(artifact): path
            for artifact, path in exported.items()
        }

    launcher_env = {
        **accelerator_env(),
        "ACCELERATOR_CACHE_DIR": str(_RUNTIME_TREE_CACHE),
    }
    ensured = context.run(
        f"{shlex.quote(launcher_env['ACCELERATOR_BIN'])} cache ensure "
        + " ".join(_RUNTIME_TREES),
        warn=True,
        hide=True,
        env=launcher_env,
    )
    resolved: dict[str, str] = {}
    launcher_said: list[str] = []
    if ensured is not None and ensured.exited == 0:
        for line in ensured.stdout.splitlines():
            fields = line.split("\t")
            if len(fields) >= 2 and fields[0] in _RUNTIME_TREES:
                resolved[fields[0]] = fields[1]
    elif ensured is not None:
        launcher_said = [f"The launcher said:\n  {ensured.stderr.strip()}"]
    for artifact in _RUNTIME_TREES:
        tree = resolved.get(artifact)
        if tree is None or not Path(tree).is_dir():
            raise Exit(
                "\n".join(
                    [
                        f"the vendored Playwright {artifact} tree is not "
                        "available.",
                        *launcher_said,
                        "Fix the cause, then re-run "
                        "`mise run test:integration:design-automation`. This "
                        "lane fails rather than skipping: a skipped runtime "
                        "suite is indistinguishable from a passing one.",
                    ]
                ),
                code=1,
            )
    return {
        _tree_variable(artifact): resolved[artifact]
        for artifact in _RUNTIME_TREES
    }


@task(help={"filter_": NODE_TEST_RUNTIME_FILTER_HELP})
def design_automation(context: Context, filter_: str | None = None) -> None:
    """Run the Playwright-executor suites against the vendored runtime.

    Rather than skip without a runtime — which is indistinguishable from
    passing — this refuses up front, following the `docker info` preflight
    precedent in `tasks/test/e2e.py`.
    """
    selection = selection_from(filter_)
    root = repo_root()
    runtime_trees = _resolve_runtime_trees(context)

    suites = [
        root / _PLAYWRIGHT_DIR / name
        for name in _DESIGN_AUTOMATION_RUNTIME_SUITES
    ]
    missing = [str(path) for path in suites if not path.exists()]
    if missing:
        raise Exit(
            f"named runtime suite(s) missing: {', '.join(missing)}", code=1
        )

    for suite in suites:
        offenders = _bare_returns_in_tests(suite.read_text())
        if offenders:
            listed = "\n  ".join(offenders)
            raise Exit(
                f"{suite.name} returns early inside a test body, which "
                f"`node --test` reports as passed rather than skipped:\n  "
                f"{listed}",
                code=1,
            )

    discovered = " ".join(str(path) for path in suites)
    result = context.run(
        command_line(
            "node --test --test-reporter=tap", selection.node_test(), discovered
        ),
        warn=True,
        pty=False,
        env=runtime_trees,
    )
    counts = _tap_counts(result.stdout)
    if counts["fail"]:
        raise Exit(
            f"{counts['fail']} design-automation runtime test(s) failed", code=1
        )
    if selection.is_filtered:
        report_filtered_design_automation_run(
            selection,
            "test:integration:design-automation",
            result.stdout,
            suites,
        )
        return
    if counts["skipped"]:
        raise Exit(
            f"{counts['skipped']} design-automation runtime test(s) skipped; "
            "this lane's preflight guarantees a runtime, so a skip means a "
            "suite is gating itself on something it should not need",
            code=1,
        )
    if counts["pass"] < _EXPECTED_DESIGN_AUTOMATION_RUNTIME_CASES:
        raise Exit(
            f"expected at least {_EXPECTED_DESIGN_AUTOMATION_RUNTIME_CASES} "
            f"executed runtime cases, got {counts['pass']}",
            code=1,
        )
