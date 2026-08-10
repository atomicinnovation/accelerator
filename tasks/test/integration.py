import os
import shlex
import tempfile
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.paths import CARGO_TOML, CLI_WORKSPACE_CARGO_TOML

from .helpers import accelerator_env, run_shell_suites

# An env gate rather than a `--yes` flag: a task name can be tab-completed by
# accident, and the CI step sets this in `env:`.
_SHADOW_OPT_IN = "ACCELERATOR_ZERO_SPAWN_SHADOW"

# Hands the prebuilt fixture matrix to the suite, which cannot build one once
# the binaries are out of reach. Mirrors fixtures::MATRIX_ROOT_VARIABLE.
_MATRIX_ROOT = "ACCELERATOR_ZERO_SPAWN_MATRIX"

# Checked alongside every PATH hit and `mise which`, because absolute-path
# access is what the strong form has to defeat.
_ABSOLUTE_VCS_PATHS = (
    "/usr/bin/git",
    "/usr/local/bin/git",
    "/opt/homebrew/bin/git",
    "/usr/bin/jj",
    "/usr/local/bin/jj",
    "/opt/homebrew/bin/jj",
)

# The config subtree (scripts/) discoverable shell suites. This is an
# at-least floor so a dropped exec bit on a fail-closed gate (e.g.
# test-skill-frontmatter-conformance.sh) can't silently vanish from CI.
# Bumped as suites are added under scripts/. Dropped from 21 to 18 as
# test-skills-index.sh (superseded by docs:generate) and
# test-config.sh/test-config-read-doc-type-paths.sh (retired alongside the
# removal set) went away. Dropped to 16 as the shell-based frontmatter and
# linkage validators retired in favour of `accelerator corpus`. Dropped to 15
# as the meta-directory migration engine's interactive wire-protocol harness
# retired in favour of the native accelerator-migrate port.
_EXPECTED_CONFIG_SUITES = 15

# The skills/work subtree discoverable shell suites. At-least floor (mirror of
# the migrate/config guards) so a dropped exec bit can't silently shrink the
# regression net. Bumped as suites are added under skills/work (pattern,
# scripts, create-remote, fetch-remote, update-remote, sync-apply).
_EXPECTED_WORK_SUITES = 5

# The skills/integrations subtree discoverable shell suites (every individual
# test-jira-*.sh + test-linear-*.sh; the test-jira-scripts.sh umbrella runner is
# excluded from discovery — see EXCLUDED_HELPER_NAMES). At-least floor so a
# dropped exec bit can't silently drop a create/auth suite from CI.
_EXPECTED_INTEGRATIONS_SUITES = 32

# Fail-closed gates that MUST run by name, not merely satisfy the count floor —
# a guard renamed off the `test-*.sh` convention would vanish while the count
# still passes via other suites. The producer-conformance guard (work item
# 0103) is the gate that "cannot drift undetected". The corpus validator's own
# migration-completion gate (work item 0102) moved with it to `corpus-cli`'s
# unconditional `this_repositorys_own_corpus_is_clean` cargo test — a `cargo
# test` failure already fails CI unconditionally, so no bash-style
# required-suite registration applies there.
_REQUIRED_CONFIG_SUITES = ("scripts/test-skill-frontmatter-conformance.sh",)

# The three previously-unguarded subtrees, each at its current size. hooks/
# holds only the one bash harness that predates Python becoming the standard
# test language (test-vcs-detect.sh — the meta-directory migration
# discoverability hook's own harness retired alongside the bash migration
# engine it gated); the link-refresh suite is pytest, where a lost file is a
# collection error rather than a silently smaller run, so no by-name entry is
# needed.
_EXPECTED_HOOKS_SUITES = 1
_EXPECTED_DECISIONS_SUITES = 0
_EXPECTED_GITHUB_SUITES = 0


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
    """Run the zero-spawn suite with git and jj shadowed.

    `PATH` stubs alone leave the binaries reachable by absolute path, so this
    moves every resolved `git`/`jj` aside and hands the harness the list, which
    hard-fails if any listed path is still executable.

    **Moves binaries out of system directories with `sudo`**, so it is gated
    behind `ACCELERATOR_ZERO_SPAWN_SHADOW=yes` and stays out of every roll-up:
    `/opt/homebrew/bin` is user-writable, and a developer running this unaware
    could be left without `git` or `jj`. Ephemeral CI runners are the host.

    Ordering is load-bearing. Compilation needs git (`vergen-gitcl`, and a cold
    registry cache) and the fixture matrix needs both binaries, so both happen
    before the window, the suite runs `--no-run` first, and the matrix is handed
    over by path for the suite to adopt. Cargo is invoked
    directly rather than through `mise run`, which could see `jj` missing inside
    the window and reinstall it, making the assertion vacuous.
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
    matrix_root = Path(tempfile.mkdtemp(prefix="accelerator-vcs-matrix-"))
    _build_fixture_matrix(context, matrix_root)

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
                # Adopted, not rebuilt: there is no git or jj to build with now.
                _MATRIX_ROOT: str(matrix_root),
            },
        )
    finally:
        _restore_vcs_binaries(context, shadow_dir, shadowed)


def _compile_zero_spawn_targets(context: Context) -> None:
    """Build the reference artefacts and compile the suite, without running."""
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


def _build_fixture_matrix(context: Context, root: Path) -> None:
    """Build the matrix at `root` while the real CLIs are still reachable.

    One test rather than the whole binary: with a shared root, tests building
    concurrently would race to populate it. The rest of the matrix suite runs in
    `test:unit:cli` against its own temp roots.
    """
    context.run(
        "cargo nextest run "
        f"--manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "-p vcs-test-support "
        "-E 'binary(matrix) and test(every_recorded_fixture_key_is_built)'",
        pty=True,
        env={_MATRIX_ROOT: str(root)},
    )


def _resolve_vcs_binaries(context: Context) -> list[Path]:
    """Resolve every `git`/`jj` on `PATH`, plus the absolute paths to check.

    Every `PATH` hit rather than the first, because macOS ships `git` in two
    directories. `mise which` is load-bearing on CI: there is no system `jj`
    there, and what sits on `PATH` may be a shim pointing into the mise install
    tree. Shadowing only the shim would leave the real binary reachable by
    absolute path while the harness agreed the run was strong.
    """
    found: list[Path] = []
    seen: set[Path] = set()

    def remember(candidate: Path) -> None:
        # Deduplicated by RESOLVED path: on a usrmerge Linux /bin is a symlink
        # to /usr/bin, so both spellings name one file and moving it under the
        # first name makes the second fail to stat.
        if not os.access(candidate, os.X_OK):
            return
        identity = candidate.resolve()
        if identity in seen:
            return
        seen.add(identity)
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

    Idempotent per path, reporting at the end rather than aborting on the first
    failure, so a partial shadow does not strand the rest. An unrestored `git`
    also breaks `actions/checkout`'s post step.
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
    suites = run_shell_suites(
        context, "scripts", accelerator_env(corpus_bin=True)
    )
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

    Two halves: the pytest suites and the one remaining bash harness that
    predates Python becoming the test language for the non-Rust surfaces.
    That harness (test-vcs-detect.sh) dispatches the compiled accelerator-vcs
    sub-binary through the real launcher, so it needs both on
    ACCELERATOR_BIN/ACCELERATOR_VCS_BIN — the vcs_bin=True overlay.
    """
    context.run("uv run pytest tests/integration/hooks -v")
    suites = run_shell_suites(context, "hooks", accelerator_env(vcs_bin=True))
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
