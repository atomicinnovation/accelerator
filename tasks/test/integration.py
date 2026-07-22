from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML

from .helpers import ensure_accelerator_bin, repo_root, run_shell_suites

# The migrate subtree ships exactly these shell suites. The count is asserted in
# `migrate` below so a dropped exec bit (e.g. on an exec-bit-lossy filesystem)
# fails the build loudly instead of silently shrinking the regression net.
_EXPECTED_MIGRATE_SUITES = 4

# The config subtree (scripts/) discoverable shell suites. Like the migrate
# guard, this is an at-least floor so a dropped exec bit on a fail-closed gate
# (e.g. validate-corpus-frontmatter.sh — the AC-1 corpus validator) can't
# silently vanish from CI. Bumped as suites are added under scripts/. Dropped
# from 21 to 19 when 0167 retired test-config.sh and
# test-config-read-doc-type-paths.sh alongside the removal set.
_EXPECTED_CONFIG_SUITES = 19

# The skills/work subtree discoverable shell suites. At-least floor (mirror of
# the migrate/config guards) so a dropped exec bit can't silently shrink the
# regression net. Bumped as suites are added under skills/work (pattern,
# scripts, create-remote, fetch-remote, update-remote, sync-apply).
_EXPECTED_WORK_SUITES = 6

# The skills/integrations subtree discoverable shell suites (every individual
# test-jira-*.sh + test-linear-*.sh; the test-jira-scripts.sh umbrella runner is
# excluded from discovery — see EXCLUDED_HELPER_NAMES). At-least floor so a
# dropped exec bit can't silently drop a create/auth suite from CI.
_EXPECTED_INTEGRATIONS_SUITES = 32

# Fail-closed gates that MUST run by name, not merely satisfy the count floor —
# a guard renamed off the `test-*.sh` convention would vanish while the count
# still passes via other suites. The producer-conformance guard (work item
# 0103) is the gate that "cannot drift undetected"; the corpus validator (work
# item 0102) hosts the migration-completion gate (its whole-corpus sanity run is
# the migration-complete signal). Both presences are asserted by identity.
_REQUIRED_CONFIG_SUITES = (
    "scripts/test-skill-frontmatter-conformance.sh",
    "scripts/test-validate-corpus-frontmatter.sh",
)


@task
def visualiser(context: Context) -> None:
    """Integration tests for the visualiser (cargo --tests + shell suites).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature, so the cargo invocation enables that feature to include it.
    """
    # Build the launcher and export ACCELERATOR_BIN/CLAUDE_PLUGIN_ROOT first:
    # the cargo tests include config_contract.rs, which runs the repointed
    # write-visualiser-config.sh, so they need the compiled launcher on the env
    # rather than the signed-release bootstrap.
    ensure_accelerator_bin(context)
    context.run(
        f"cargo test --manifest-path {CARGO_TOML} --tests "
        f"--no-default-features --features dev-frontend"
    )
    run_shell_suites(context, "skills/visualisation/visualise")


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
def deny(context: Context) -> None:
    """cargo-deny native-tls/OpenSSL ban regression (offline fixtures)."""
    context.run("uv run pytest tests/integration/deny -v")


@task
def pup(context: Context) -> None:
    """cargo-pup architecture regression (needs the nightly lane)."""
    context.run("uv run pytest tests/integration/pup -v")


@task
def config(context: Context) -> None:
    """Integration tests for the plugin-wide config scripts."""
    ensure_accelerator_bin(context)
    suites = run_shell_suites(context, "scripts")
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
def decisions(context: Context) -> None:
    """Integration tests for the decisions skill scripts."""
    ensure_accelerator_bin(context)
    run_shell_suites(context, "skills/decisions")


@task
def binary_acquisition(context: Context) -> None:
    """Test launch-server.sh binary acquisition (sentinel, SHA, 404)."""
    ensure_accelerator_bin(context)
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
def work(context: Context) -> None:
    """Integration tests for the work-management skill scripts."""
    ensure_accelerator_bin(context)
    suites = run_shell_suites(context, "skills/work")
    if len(suites) < _EXPECTED_WORK_SUITES:
        raise Exit(
            f"Expected at least {_EXPECTED_WORK_SUITES} work shell suites, "
            f"found {len(suites)}: {suites}. An exec bit may have been "
            f"dropped — a work-management regression suite is missing from CI.",
            code=1,
        )


@task
def integrations(context: Context) -> None:
    """Integration tests for the jira/linear integration scripts."""
    ensure_accelerator_bin(context)
    suites = run_shell_suites(context, "skills/integrations")
    if len(suites) < _EXPECTED_INTEGRATIONS_SUITES:
        raise Exit(
            f"Expected at least {_EXPECTED_INTEGRATIONS_SUITES} integration "
            f"shell suites, found {len(suites)}: {suites}. An exec bit may "
            f"have been dropped — a create/auth suite is missing from CI.",
            code=1,
        )


@task
def migrate(context: Context) -> None:
    """Integration tests for the meta-directory migration framework."""
    ensure_accelerator_bin(context)
    suites = run_shell_suites(context, "skills/config/migrate")
    if len(suites) < _EXPECTED_MIGRATE_SUITES:
        raise Exit(
            f"Expected at least {_EXPECTED_MIGRATE_SUITES} migrate shell "
            f"suites, found {len(suites)}: {suites}. An exec bit may have "
            f"been dropped — the migrate regression net is incomplete.",
            code=1,
        )
