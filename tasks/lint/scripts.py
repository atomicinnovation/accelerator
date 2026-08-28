import re
import shlex
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import SURVIVING_SHELL_SOURCES, repo_root

# An empty scan set means the survivor list was emptied by mistake, not that
# there is nothing to lint — so every task below fails loudly (fail-closed).
_EMPTY_SCOPE = "no shell sources matched — scope discovery is broken"


@task
def shellcheck(context: Context) -> None:
    """Lint the surviving shell with ShellCheck (config in .shellcheckrc)."""
    if not SURVIVING_SHELL_SOURCES:
        raise Exit(f"shellcheck: {_EMPTY_SCOPE}", code=1)
    args = " ".join(shlex.quote(s) for s in SURVIVING_SHELL_SOURCES)
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
    if not SURVIVING_SHELL_SOURCES:
        raise Exit(f"bashisms: {_EMPTY_SCOPE}", code=1)
    findings = scan_bashisms(list(SURVIVING_SHELL_SOURCES), repo_root())
    if findings:
        listed = "\n  ".join(findings)
        raise Exit(
            f"lint-bashisms found bash-4 constructs:\n  {listed}", code=1
        )
