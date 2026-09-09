import os
import shlex
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML
from tasks.test.cli import _MANIFEST

from .helpers import accelerator_env, repo_root


@task
def visualiser(context: Context) -> None:
    """Integration tests for the visualiser (cargo --tests).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature, so the cargo invocation enables that feature to include it.
    """
    # The cargo tests include orchestration_lifecycle.rs, which dispatches the
    # compiled launcher, so they need it on the env (built by the build:cli:dev
    # mise dependency) rather than the signed-release bootstrap. The overlay is
    # passed to each child, not written into this process.
    env = accelerator_env()
    context.run(
        f"cargo test --manifest-path {CARGO_TOML} --tests "
        f"--no-default-features --features dev-frontend",
        env=env,
    )


@task
def dev(context: Context) -> None:
    """Integration tests for the dev task (real circusd, fake processes)."""
    context.run("uv run pytest tests/integration/dev -v")


@task
def entrypoint(context: Context) -> None:
    """Hermetic tests for the bin/accelerator plugin entry point.

    Exercises the bootstrap end-to-end against a stubbed downloader and real
    minisign signatures verified by the real accelerator-verify shim.
    """
    context.run("uv run pytest tests/integration/entrypoint -v")


@task
def skill_invocation(context: Context) -> None:
    """Run every SKILL.md `!`-site config command in the production shape."""
    context.run("uv run pytest tests/integration/skill-invocation -v")


@task
def deny(context: Context) -> None:
    """cargo-deny native-tls/OpenSSL ban regression (offline fixtures)."""
    context.run("uv run pytest tests/integration/deny -v")


@task
def pup(context: Context) -> None:
    """cargo-pup architecture regression (needs the nightly lane)."""
    context.run("uv run pytest tests/integration/pup -v")


@task
def tracker_contract(context: Context) -> None:
    """Run the RemoteTracker contract harness (excluded from test:unit:cli)."""
    context.run(
        f"cargo nextest run --profile contract {_MANIFEST}",
        env={"ACCELERATOR_TRACKER_CONTRACT": "1"},
        pty=True,
    )


@task
def conformance(context: Context) -> None:
    """Producer-conformance guard: drives the real corpus validator.

    Runs the launcher-provisioning pytest lane with ACCELERATOR_CORPUS_BIN
    overlaid (via ``accelerator_env(corpus_bin=True)``) and the launcher built
    by the ``build:cli:dev`` mise dependency, so it dispatches
    ``accelerator corpus frontmatter validate`` to the compiled sub-binary.
    """
    context.run(
        "uv run pytest tests/integration/conformance -v",
        env=accelerator_env(corpus_bin=True),
    )


@task
def hooks(context: Context) -> None:
    """Integration tests for the hooks/ subtree.

    The launcher-dispatch smoke drives the compiled accelerator-vcs through the
    real `bin/accelerator` wrapper, so it needs ACCELERATOR_VCS_BIN — the
    vcs_bin=True overlay — and the launcher built by the build:cli:dev
    dependency.
    """
    context.run(
        "uv run pytest tests/integration/hooks -v",
        env=accelerator_env(vcs_bin=True),
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

# The launcher exports this on a tree-consuming dispatch; the lane reuses it
# when present rather than paying a second materialisation.
_DRIVER_TREE_ENV = "ACCELERATOR_TREE_DRIVER"


def _resolve_driver_tree(context: Context) -> Path:
    """Resolve the materialised driver tree, or refuse the lane.

    Prefers the launcher-exported driver-tree path; failing that, materialises
    it through `accelerator cache ensure driver` and reads the tree path from
    the tab-separated success line. An unresolvable runtime is a visible refusal
    rather than a silent pass.
    """
    exported = os.environ.get(_DRIVER_TREE_ENV)
    if exported and Path(exported).is_dir():
        return Path(exported)

    accelerator = repo_root() / "bin/accelerator"
    ensured = context.run(
        f"{shlex.quote(str(accelerator))} cache ensure driver",
        warn=True,
        hide=True,
    )
    tree: Path | None = None
    if ensured is not None and ensured.exited == 0:
        for line in ensured.stdout.splitlines():
            fields = line.split("\t")
            if len(fields) >= 2 and fields[0] == "driver":
                tree = Path(fields[1])
                break
    if tree is None or not tree.is_dir():
        raise Exit(
            "the vendored Playwright driver tree is not available. Run "
            "`accelerator cache ensure driver`, then retry. This lane fails "
            "rather than skipping: a skipped runtime suite is "
            "indistinguishable from a passing one.",
            code=1,
        )
    return tree


@task
def design_automation(context: Context) -> None:
    """Run the Playwright-executor suites that need a real runtime (opt-in).

    Deliberately outside `test:integration` and the bare default task: no CI
    lane provisions a Playwright runtime. Rather than skip without one — which
    is indistinguishable from passing — this refuses up front, following the
    `docker info` preflight precedent in `tasks/test/e2e.py`.
    """
    root = repo_root()
    driver_tree = _resolve_driver_tree(context)

    suites = [
        root / _PLAYWRIGHT_DIR / name
        for name in _DESIGN_AUTOMATION_RUNTIME_SUITES
    ]
    missing = [str(path) for path in suites if not path.exists()]
    if missing:
        raise Exit(
            f"named runtime suite(s) missing: {', '.join(missing)}", code=1
        )

    discovered = " ".join(str(path) for path in suites)
    result = context.run(
        f"node --test {discovered}",
        warn=True,
        pty=False,
        env={_DRIVER_TREE_ENV: str(driver_tree)},
    )
    if result.exited != 0:
        raise Exit("design-automation runtime tests failed", code=1)
