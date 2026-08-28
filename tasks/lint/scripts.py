import os
import re
import shlex
from pathlib import Path

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
        "scripts/test-helpers.sh",
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


# The bash-4 denylist, translated construct-for-construct from the retired
# lint-bashisms.sh awk source. POSIX classes map to explicit ASCII ranges under
# re.ASCII so `\w`/`\d`/`\s` never admit `_`, Unicode digits, or Unicode
# whitespace the C-locale awk did not: [[:alpha:]]->[A-Za-z], [[:alnum:]_...]->
# [A-Za-z0-9_...], [[:digit:]]->[0-9]. [[:space:]]->[ \t] is a deliberate
# narrowing (a keyword separated from its flag by a form-feed is not realistic).
# First match wins, mirroring the awk if/else-if chain.
_BASHISM_PATTERNS: list[tuple[re.Pattern[str], str]] = [
    (
        re.compile(r"(declare|local|typeset)[ \t]+-A", re.ASCII),
        "associative array (declare/local/typeset -A)",
    ),
    (
        re.compile(r"(declare|local|typeset)[ \t]+-[A-Za-z]*n", re.ASCII),
        "nameref (declare/local/typeset -n)",
    ),
    (
        re.compile(r"\$\{[^}]*:[-=+?][^}]*\\[{}]", re.ASCII),
        "escaped brace in parameter-expansion default (bash 3.2 keeps the "
        "backslash)",
    ),
    (
        re.compile(
            r"(^|[^A-Za-z0-9_])(mapfile|readarray)([^A-Za-z0-9_]|$)", re.ASCII
        ),
        "mapfile/readarray",
    ),
    (
        re.compile(r"\$\{[A-Za-z0-9_\[\]@*]+(\^|,)", re.ASCII),
        "case-modification expansion (^^ ^ ,, ,)",
    ),
    (re.compile(r"&>>", re.ASCII), "&>> append-both redirect"),
    (re.compile(r"\|&", re.ASCII), "|& pipe-both"),
    (re.compile(r"\[-[0-9]", re.ASCII), "negative array subscript"),
]

# Tested against the raw, unstripped line, matching the awk rule that runs
# before the comment strip.
_OPT_OUT = re.compile(r"# lint-bashisms: ignore([ \t]|$)")

# The naive, quote-unaware trailing-comment strip the awk source used; a single
# substitution, deliberately not "improved".
_TRAILING_COMMENT = re.compile(r"(^|[ \t])#.*$")


def scan_bashisms(sources: list[str], root: Path) -> list[str]:
    r"""Return one `<file>:<line>: bash-4 construct: <msg>` per denylist hit.

    Each source is read as UTF-8 (the scripts carry em-dashes; the locale
    default would raise under a forced ``LANG=C``) and split on ``"\\n"`` — not
    ``splitlines()``, which also breaks on form-feeds and the Unicode line
    separators, shifting the line numbers awk's ``RS="\\n"`` assigns.
    """
    findings: list[str] = []
    for rel in sources:
        text = (root / rel).read_text(encoding="utf-8")
        for number, line in enumerate(text.split("\n"), start=1):
            if _OPT_OUT.search(line):
                continue
            code = _TRAILING_COMMENT.sub("", line, count=1)
            for pattern, message in _BASHISM_PATTERNS:
                if pattern.search(code):
                    findings.append(
                        f"{rel}:{number}: bash-4 construct: {message}"
                    )
                    break
    return findings


@task
def bashisms(context: Context) -> None:
    """Guard the bash-3.2 floor by scanning for denylisted bash-4 constructs."""
    sources = shell_sources()
    if not sources:
        raise Exit(f"bashisms: {_EMPTY_SCOPE}", code=1)
    findings = scan_bashisms(sources, repo_root())
    if findings:
        listed = "\n  ".join(findings)
        raise Exit(
            f"lint-bashisms found bash-4 constructs:\n  {listed}", code=1
        )


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
