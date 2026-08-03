import os
import shlex
import tempfile
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML, CLI_WORKSPACE_CARGO_TOML

from .helpers import accelerator_env, run_shell_suites

# The strong-form zero-spawn run moves system binaries aside with sudo, so it
# refuses to start without this. Not a `--yes` flag: the CI step sets it in
# `env:`, and an env gate cannot be reached by tab-completing a task name.
_SHADOW_OPT_IN = "ACCELERATOR_ZERO_SPAWN_SHADOW"

# Checked in addition to every PATH hit and to `mise which`, because a probe
# reaching for one of these by absolute path is exactly what the strong form has
# to defeat. These are the system locations; on CI there is no system jj at all,
# which is why `mise which` carries the jj side.
_ABSOLUTE_VCS_PATHS = (
    "/usr/bin/git",
    "/usr/local/bin/git",
    "/opt/homebrew/bin/git",
    "/usr/bin/jj",
    "/usr/local/bin/jj",
    "/opt/homebrew/bin/jj",
)

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

# The three previously-unguarded subtrees, each at its current size. hooks/
# holds only the two bash harnesses that predate ADR-0048; the link-refresh
# suite is pytest, where a lost file is a collection error rather than a
# silently smaller run, so no by-name entry is needed.
_EXPECTED_HOOKS_SUITES = 2
_EXPECTED_DECISIONS_SUITES = 1
_EXPECTED_GITHUB_SUITES = 3


def _require_suite_floor(
    suites: list[str],
    floor: int,
    required: tuple[str, ...],
    subject: str,
) -> None:
    """Fail loudly when discovery shrinks below its floor or loses a gate.

    An exec bit dropped on an exec-bit-lossy filesystem, or a suite renamed off
    the ``test-*.sh`` convention, otherwise removes a regression net from CI
    while every task still exits 0.
    """
    if len(suites) < floor:
        raise Exit(
            f"Expected at least {floor} {subject} shell suites, found "
            f"{len(suites)}: {suites}. An exec bit may have been dropped — a "
            f"regression suite is missing from CI.",
            code=1,
        )
    missing = [s for s in required if s not in suites]
    if missing:
        raise Exit(
            f"Required {subject} shell suite(s) not discovered by name: "
            f"{missing} (found {suites}). A gate may have lost its exec bit or "
            f"been renamed off the test-*.sh convention.",
            code=1,
        )


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
def zero_spawn(context: Context) -> None:
    """Run the zero-spawn suite (its own CI job, not the test roll-up).

    Builds the reference artefact first: the suite resolves it beside its own
    test binary, and a bare `cargo nextest run -p corpus-adapters` does not
    build another crate's bin target.
    """
    context.run(
        "cargo build "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p vcs-adapters --bin vcs-adapters-fixture",
        pty=True,
    )
    context.run(
        "cargo nextest run "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p corpus-adapters --features bash-parity -E 'binary(zero_spawn)'",
        pty=True,
    )


@task
def zero_spawn_strong(context: Context) -> None:
    """Run the zero-spawn suite in its strong form, with git and jj shadowed.

    The strong form is what makes the property non-degradable: `PATH` stubs
    alone leave the binaries reachable by absolute path, so this moves every
    resolved `git`/`jj` aside for the duration of the run and hands the harness
    the list, which hard-fails if any listed path is still executable.

    **This moves binaries out of system directories and needs `sudo`.** It is
    gated behind `ACCELERATOR_ZERO_SPAWN_SHADOW=yes` and stays out of every
    roll-up, because `/opt/homebrew/bin` is user-writable — a developer who ran
    this unaware could be left without `git` or `jj`. Ephemeral CI runners are
    the intended host; `tasks/README.md` records the containment assumption.

    Ordering is load-bearing. Everything needing a real binary happens *before*
    the shadow window: `cli/launcher` carries a `vergen-gitcl` build dependency
    that shells out to git, cargo may need git on a cold registry cache, and the
    fixture matrix is built by invoking the real CLIs. The suite is therefore
    compiled with `--no-run` first and only executed inside the window, and
    cargo is invoked directly rather than through `mise run` — mise could
    observe `jj` as missing inside the window and reinstall it, silently
    restoring the binary and making the assertion vacuous.
    """
    if os.environ.get(_SHADOW_OPT_IN) != "yes":
        raise Exit(
            f"refusing to shadow the real git/jj: set {_SHADOW_OPT_IN}=yes to "
            "confirm. This moves binaries out of system directories with sudo "
            "and is meant for ephemeral CI runners, not a developer machine. "
            "For the local property, run test:integration:zero-spawn instead.",
            code=1,
        )

    _compile_zero_spawn_targets(context)
    _build_fixture_matrix(context)

    targets = _resolve_vcs_binaries(context)
    if not targets:
        raise Exit("found no git or jj to shadow — nothing to prove", code=1)

    shadow_dir = Path(tempfile.mkdtemp(prefix="accelerator-shadowed-"))
    shadowed: list[Path] = []
    try:
        for target in targets:
            stashed = shadow_dir / str(target).replace(os.sep, "_")
            context.run(f"sudo mv {shlex.quote(str(target))} {stashed}")
            shadowed.append(target)
        context.run(
            "cargo nextest run "
            f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
            "-p corpus-adapters --features bash-parity "
            "-E 'binary(zero_spawn)' --no-fail-fast",
            pty=True,
            env={
                "ACCELERATOR_ZERO_SPAWN_MODE": "strong",
                "ACCELERATOR_ZERO_SPAWN_SHADOWED": ":".join(
                    str(path) for path in shadowed
                ),
            },
        )
    finally:
        _restore_vcs_binaries(context, shadow_dir, shadowed)


def _compile_zero_spawn_targets(context: Context) -> None:
    """Build the reference artefacts and compile the suite without running it.

    Both need a real git, so both happen before the shadow window.
    """
    context.run(
        "cargo build "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p vcs-adapters --bin vcs-adapters-fixture "
        "--bin vcs-adapters-fixture-stub",
        pty=True,
    )
    context.run(
        "cargo nextest run "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p corpus-adapters --features bash-parity "
        "-E 'binary(zero_spawn)' --no-run",
        pty=True,
    )


def _build_fixture_matrix(context: Context) -> None:
    """Exercise the matrix builders while the real CLIs are still reachable."""
    context.run(
        "cargo nextest run "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p vcs-test-support -E 'binary(matrix)'",
        pty=True,
    )


def _resolve_vcs_binaries(context: Context) -> list[Path]:
    """Resolve every `git`/`jj` on `PATH`, plus the absolute paths to check.

    Every `PATH` hit rather than the first: macOS ships `git` in two
    directories, and the enumeration keeps this honest when a runner image
    moves things. Deduplicated, order preserved, and only what exists.

    `mise which` is asked as well, and it is the load-bearing one on CI. There
    is no system `jj` on the runner at all — the real binary lives under the
    mise install tree, and what sits on `PATH` may be a shim pointing at it.
    Shadowing only the shim would leave the real binary reachable by absolute
    path, which is precisely what the strong form exists to defeat, while the
    harness would agree the run was strong because we only told it about the
    shim.
    """
    found: list[Path] = []

    def remember(candidate: Path) -> None:
        if os.access(candidate, os.X_OK) and candidate not in found:
            found.append(candidate)

    for name in ("git", "jj"):
        for directory in os.environ.get("PATH", "").split(os.pathsep):
            if directory:
                remember(Path(directory) / name)
        resolved = context.run(f"mise which {name}", warn=True, hide=True)
        if resolved is not None and resolved.exited == 0:
            target = resolved.stdout.strip()
            if target:
                remember(Path(target))
    for absolute in _ABSOLUTE_VCS_PATHS:
        remember(Path(absolute))
    return found


def _restore_vcs_binaries(
    context: Context, shadow_dir: Path, shadowed: list[Path]
) -> None:
    """Put every shadowed binary back, then prove it is runnable again.

    Idempotent per path and it reports at the end rather than aborting on the
    first failure, so a partial shadow does not leave the rest stranded. Raises
    when anything is still missing: an unrestored `git` also breaks
    `actions/checkout`'s post step, which runs git to strip its auth token.
    """
    still_missing: list[Path] = []
    for target in shadowed:
        stashed = shadow_dir / str(target).replace(os.sep, "_")
        if stashed.exists():
            context.run(
                f"sudo mv -f {stashed} {shlex.quote(str(target))}", warn=True
            )
        if not os.access(target, os.X_OK):
            still_missing.append(target)
    if still_missing:
        raise Exit(
            "failed to restore: "
            + ", ".join(str(path) for path in still_missing),
            code=1,
        )


@task
def config(context: Context) -> None:
    """Integration tests for the plugin-wide config scripts."""
    suites = run_shell_suites(context, "scripts", accelerator_env())
    _require_suite_floor(
        suites, _EXPECTED_CONFIG_SUITES, _REQUIRED_CONFIG_SUITES, "config"
    )


@task
def decisions(context: Context) -> None:
    """Integration tests for the decisions skill scripts."""
    suites = run_shell_suites(context, "skills/decisions", accelerator_env())
    _require_suite_floor(suites, _EXPECTED_DECISIONS_SUITES, (), "decisions")


@task
def hooks(context: Context) -> None:
    """Integration tests for the hooks/ subtree.

    Two halves: the pytest suites (ADR-0048 — Python is the test language for
    the non-Rust surfaces) and the two bash harnesses that predate it.
    """
    context.run("uv run pytest tests/integration/hooks -v")
    suites = run_shell_suites(context, "hooks")
    _require_suite_floor(suites, _EXPECTED_HOOKS_SUITES, (), "hooks")


@task
def github(context: Context) -> None:
    """Integration tests for the github skills (shell harnesses)."""
    suites = run_shell_suites(context, "skills/github")
    _require_suite_floor(suites, _EXPECTED_GITHUB_SUITES, (), "github")


@task
def work(context: Context) -> None:
    """Integration tests for the work-management skill scripts."""
    suites = run_shell_suites(context, "skills/work", accelerator_env())
    _require_suite_floor(suites, _EXPECTED_WORK_SUITES, (), "work")


@task
def integrations(context: Context) -> None:
    """Integration tests for the jira/linear integration scripts."""
    suites = run_shell_suites(context, "skills/integrations", accelerator_env())
    _require_suite_floor(
        suites, _EXPECTED_INTEGRATIONS_SUITES, (), "integrations"
    )


@task
def migrate(context: Context) -> None:
    """Integration tests for the meta-directory migration framework."""
    suites = run_shell_suites(
        context, "skills/config/migrate", accelerator_env()
    )
    _require_suite_floor(suites, _EXPECTED_MIGRATE_SUITES, (), "migrate")
