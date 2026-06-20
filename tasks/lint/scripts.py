import os
import shlex

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root, shell_sources

# An empty match set means scope discovery broke (a glob/`_keep` regression),
# not that there is nothing to lint — so every task below fails loudly rather
# than passing green (fail-closed, not fail-open).
_EMPTY_SCOPE = "no shell sources matched — scope discovery is broken"

# Sourced-only shell libraries: loaded via `source`/`.`, never invoked by path.
# The guard enforces *executable iff NOT on this list*, so a tracked .sh absent
# here is treated as an entrypoint and must be 0755. A NEW sourced-only library
# MUST be added here or the guard will demand +x on it. See the
# "Executable-bit invariant" subsection in tasks/README.md.
SHELL_LIBRARIES: frozenset[str] = frozenset(
    {
        "scripts/fs-common.sh",
        "scripts/hash-common.sh",
        "scripts/jsonl-common.sh",
        "scripts/log-common.sh",
        "scripts/work-common.sh",
        "scripts/config-defaults.sh",
        "scripts/config-common.sh",
        "scripts/atomic-common.sh",
        "scripts/vcs-common.sh",
        "scripts/doc-type-table.sh",
        "scripts/doc-type-inference.sh",
        "scripts/frontmatter-emission-rules.sh",
        "scripts/frontmatter-fixtures.sh",
        "scripts/interactive-harness.sh",
        "scripts/interactive-protocol.sh",
        "scripts/test-helpers.sh",
        "scripts/accelerator-scaffold.sh",
        "skills/config/migrate/scripts/interactive-lib.sh",
        "skills/github/scripts/test-helpers.sh",
        "skills/visualisation/visualise/scripts/launcher-helpers.sh",
        "skills/visualisation/visualise/scripts/test-helpers.sh",
        "skills/work/scripts/work-item-common.sh",
        "skills/work/scripts/work-item-bridge-codes.sh",
        "skills/integrations/jira/scripts/jira-common.sh",
        "skills/integrations/jira/scripts/jira-auth.sh",
        "skills/integrations/jira/scripts/jira-jql.sh",
        "skills/integrations/jira/scripts/jira-body-input.sh",
        "skills/integrations/jira/scripts/jira-custom-fields.sh",
        "skills/integrations/linear/scripts/linear-common.sh",
        "skills/integrations/linear/scripts/linear-auth.sh",
    }
)

# Bash-run migration fixtures: discovered by name and executed via `bash "$f"`
# (never by exec bit, never sourced), so they are neither entrypoints nor
# libraries. Exempt from the invariant in both directions. The exemption is a
# path-segment match because shell_sources() returns POSIX-relative paths
# (see tasks/shared/sources.py); a future second fixture root would need adding
# here. A test asserts this segment matches only the known fixture tree.
_FIXTURE_SEGMENT = "test-fixtures"


def _sources_args() -> str | None:
    sources = shell_sources()
    if not sources:
        return None
    return " ".join(shlex.quote(s) for s in sources)


@task
def shellcheck(context: Context) -> None:
    """Lint every shell source with ShellCheck (config in .shellcheckrc)."""
    args = _sources_args()
    if args is None:
        raise Exit(f"shellcheck: {_EMPTY_SCOPE}", code=1)
    with context.cd(str(repo_root())):
        result = context.run(f"shellcheck {args}", warn=True, pty=False)
    if result.exited != 0:
        raise Exit(
            "shellcheck reported findings — fix them, or add a justified "
            "`# shellcheck disable=`/`source=` directive",
            code=1,
        )


@task
def bashisms(context: Context) -> None:
    """Guard the bash-3.2 floor by scanning for denylisted bash-4 constructs."""
    args = _sources_args()
    if args is None:
        raise Exit(f"bashisms: {_EMPTY_SCOPE}", code=1)
    with context.cd(str(repo_root())):
        result = context.run(
            f"bash scripts/lint-bashisms.sh {args}", warn=True, pty=False
        )
    if result.exited != 0:
        raise Exit("lint-bashisms found bash-4 constructs", code=1)


@task
def exec_bits(context: Context) -> None:
    """Enforce: a tracked .sh is executable iff NOT on SHELL_LIBRARIES."""
    sources = shell_sources()
    if not sources:
        raise Exit(f"exec-bits: {_EMPTY_SCOPE}", code=1)

    repo = repo_root()
    in_scope = set(sources)
    offenders: list[str] = []

    # Stale-entry guard: every library-list path must still be enumerated by
    # shell_sources(). Keying on `in_scope` (not mere on-disk existence) closes
    # the gap where a library that exists but has left scope — gitignored,
    # relocated under workspaces/, or lost its .sh extension — would otherwise
    # pass the existence check yet never be mode-checked below.
    offenders.extend(
        f"stale library-list entry (not enumerated): {rel}  "
        "-> remove from SHELL_LIBRARIES or restore the file"
        for rel in sorted(SHELL_LIBRARIES)
        if rel not in in_scope
    )

    for rel in sources:
        if _FIXTURE_SEGMENT in rel.split("/"):
            continue
        executable = os.access(repo / rel, os.X_OK)
        # Each line is a runnable chmod; the "then commit" reminder is in the
        # per-offender comment (not only the preamble) because the working-copy
        # bit alone does not satisfy CI — see the Working-copy-mode stance.
        # Keep the command itself paste-safe (no fake `&& commit` that errors).
        if rel in SHELL_LIBRARIES and executable:
            offenders.append(f"chmod -x {rel}  # library -> 0644, then commit")
        elif rel not in SHELL_LIBRARIES and not executable:
            offenders.append(
                f"chmod +x {rel}  # entrypoint -> 0755, then commit"
            )

    if offenders:
        raise Exit(
            "exec-bit invariant violated (a tracked .sh is executable iff it "
            "is NOT a sourced-only library). Run each line below AND COMMIT "
            "the mode change (shell has no autofixer; the bit must be "
            "committed to satisfy CI). If you believe a file is "
            'mis-classified, see the "Executable-bit invariant" subsection '
            "in tasks/README.md:"
            "\n  " + "\n  ".join(offenders),
            code=1,
        )
