from invoke import Context, Exit, task

from .helpers import repo_root, run_shell_suites

# The migrate subtree ships exactly these shell suites. The count is asserted in
# `migrate` below so a dropped exec bit (e.g. on an exec-bit-lossy filesystem)
# fails the build loudly instead of silently shrinking the regression net.
_EXPECTED_MIGRATE_SUITES = 4

# The config subtree (scripts/) discoverable shell suites. Like the migrate
# guard, this is an at-least floor so a dropped exec bit on a fail-closed gate
# (e.g. validate-corpus-frontmatter.sh — the AC-1 corpus validator) can't
# silently vanish from CI. Bumped as suites are added under scripts/ (the most
# recent bump added test-config-parity.sh, the byte-for-byte parity gate).
_EXPECTED_CONFIG_SUITES = 17

# Fail-closed gates that MUST run by name, not merely satisfy the count floor —
# a guard renamed off the `test-*.sh` convention would vanish while the count
# still passes via other suites. The producer-conformance guard (work item
# 0103) is the gate that "cannot drift undetected", so its presence is asserted
# by identity.
_REQUIRED_CONFIG_SUITES = ("scripts/test-skill-frontmatter-conformance.sh",)


@task
def visualiser(context: Context) -> None:
    """Integration tests for the visualiser (cargo --tests + shell suites).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature, so the cargo invocation enables that feature to include it.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(
        f"cargo test --manifest-path {manifest} --tests "
        f"--no-default-features --features dev-frontend"
    )
    run_shell_suites(context, "skills/visualisation/visualise")


@task
def dev(context: Context) -> None:
    """Integration tests for the dev task (real circusd, fake processes)."""
    context.run("uv run pytest tests/integration/dev -v")


def _assert_config_floor(suites: list[str]) -> None:
    """Fail loudly on a shrunken config regression net.

    Shared by the bash and a9r-parity runs so both enforce the same
    guarantee: the discovered suites meet the count floor and every
    required-by-name gate is present.
    """
    if len(suites) < _EXPECTED_CONFIG_SUITES:
        raise Exit(
            f"Expected at least {_EXPECTED_CONFIG_SUITES} config shell "
            f"suites, found {len(suites)}: {suites}. An exec bit may have "
            f"been dropped — a fail-closed gate (e.g. the corpus validator) is "
            f"missing from CI.",
            code=1,
        )
    missing = [s for s in _REQUIRED_CONFIG_SUITES if s not in suites]
    if missing:
        raise Exit(
            f"Required config shell suite(s) not discovered by name: {missing} "
            f"(found {suites}). A fail-closed gate may have lost its exec bit "
            f"or been renamed off the test-*.sh convention.",
            code=1,
        )


@task
def config(context: Context) -> None:
    """Integration tests for the plugin-wide config scripts (bash mode)."""
    _assert_config_floor(run_shell_suites(context, "scripts"))


@task(name="config-parity")
def config_parity(context: Context) -> None:
    """Re-run the config shell suites against the a9r binary (parity gate).

    The `config` task runs every scripts/ suite once in bash mode (A9R_BIN
    unset). This runs them a second time with A9R_BIN exported at the built
    binary, so `run_sut` dispatches every config-read assertion through a9r
    and `test-config-parity.sh` compares the two backends byte-for-byte. Both
    runs together are the merge gate — `mise run check` (format + lint) does
    not exercise either. The mise task depends on `build:a9r:dev`; this guard
    fails loudly if the artifact is somehow absent rather than silently
    re-testing bash.
    """
    a9r = repo_root() / "skills/visualisation/visualise/target/debug/a9r"
    if not a9r.exists():
        raise Exit(
            f"a9r binary not found at {a9r} — build it first "
            f"(mise run build:a9r:dev) so the parity run does not silently "
            f"degrade to a second bash run.",
            code=1,
        )
    suites = run_shell_suites(context, "scripts", env={"A9R_BIN": str(a9r)})
    _assert_config_floor(suites)


@task
def decisions(context: Context) -> None:
    """Integration tests for the decisions skill scripts."""
    run_shell_suites(context, "skills/decisions")


@task
def binary_acquisition(context: Context) -> None:
    """Test launch-server.sh binary acquisition (sentinel, SHA, 404)."""
    script = (
        repo_root()
        / "skills/visualisation/visualise/scripts/test-launch-server.sh"
    )
    context.run(f"bash {script}")


@task
def hooks(context: Context) -> None:
    """Integration tests for the hooks/ subtree."""
    run_shell_suites(context, "hooks")


@task
def github(context: Context) -> None:
    """Integration tests for the github skills (shell harnesses)."""
    run_shell_suites(context, "skills/github")


@task
def migrate(context: Context) -> None:
    """Integration tests for the meta-directory migration framework."""
    suites = run_shell_suites(context, "skills/config/migrate")
    if len(suites) < _EXPECTED_MIGRATE_SUITES:
        raise Exit(
            f"Expected at least {_EXPECTED_MIGRATE_SUITES} migrate shell "
            f"suites, found {len(suites)}: {suites}. An exec bit may have "
            f"been dropped — the migrate regression net is incomplete.",
            code=1,
        )
