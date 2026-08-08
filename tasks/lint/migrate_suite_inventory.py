"""Assertion inventory for the six retiring migrate/interactive test suites.

Modelled on 0167's now-lost `check-inventory.sh`, but durable (Python, under
`tasks/`, per the plan's own note that a disposable bash script was lost once
before). Extracts every `assert_*` call site — the shared library defines
them in `scripts/test-helpers.sh`, and three suites layer their own local
composite wrappers on top (`assert_validates`/`assert_violation` in
`test-migrate-0007.sh`, `assert_bridge_unmutated` in
`test-migrate-interactive.sh`) — and classifies each `repointable` (drives
the CLI surface; runs unmodified against the compiled `accelerator-migrate`
binary via a black-box wrapper) or `not-repointable` (drives FIFO/
wire-protocol internals directly).

Classification is a text heuristic, not real data-flow analysis: an
assertion's own logical statement (backslash-continuation lines joined) is
checked for a marker naming a wire-protocol internal (`$FRAME_TYPE`,
`harness_*`, `mkfifo`, ...).
`scripts/test-interactive-protocol.sh` has no CLI surface at all — every
assertion in it is `not-repointable` regardless of content, since it drives
`interactive-protocol.sh`'s functions directly rather than a subprocess.
Because this heuristic is the sole gate for Phase 10's "no gaps, no
duplicates" completeness guarantee, its own detection logic is unit-tested
against synthetic `tmp_path` trees (matching this repo's own established
convention for this kind of walk-and-classify gate,
`tests/unit/tasks/test_call_site_migration.py`) before being trusted
against the six real suites — see
`tests/unit/tasks/test_migrate_suite_inventory.py`. The known limitation
this heuristic has (it cannot detect that a whole *test*, not just an
individual assertion, has no Rust equivalent when its fixture is a
synthetic bash migration script) is recorded in
`meta/inventories/0172-suite-audit.md`, not papered over here.
"""

import re
from dataclasses import dataclass
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

# The six retiring suites, repo-relative — Technical Notes' "Source bash to
# retire" list, minus the driver/migration/harness scripts themselves (those
# have no assertions of their own to inventory).
SUITES: tuple[str, ...] = (
    "skills/config/migrate/scripts/test-migrate.sh",
    "skills/config/migrate/scripts/test-migrate-snapshot.sh",
    "skills/config/migrate/scripts/test-migrate-interactive.sh",
    "skills/config/migrate/scripts/test-migrate-0007.sh",
    "scripts/test-interactive-protocol.sh",
    "hooks/test-migrate-discoverability.sh",
)

# This suite sources `interactive-protocol.sh` and drives its functions
# in-process (`escape_field`, `emit_frame`, `read_frame`, ...) — it has no
# CLI surface to repoint at all, so every assertion in it is classified
# `not-repointable` regardless of what its own statement text mentions.
NO_CLI_SURFACE: frozenset[str] = frozenset(
    {
        "scripts/test-interactive-protocol.sh",
    }
)

# Substrings marking an assertion as exercising the FIFO/wire-protocol layer
# directly (`interactive-lib.sh`/`interactive-harness.sh`/
# `interactive-protocol.sh` internals) rather than the CLI's observable
# stdout/stderr/exit-code/file-state.
NOT_REPOINTABLE_MARKERS: tuple[str, ...] = (
    "$FRAME_TYPE",
    "$FRAME_FIELDS",
    "${FRAME_FIELDS",
    "$DECIDE_OUTCOME",
    "$DECIDE_VALUE",
    "$DECISIONS_FD",
    "$_FORK_MIG_PROTO",
    "$_FORK_RUN_PROTO",
    "harness_",
    "_interactive_fork",
    "_harness_",
    "MIGRATION_HARNESS_MODE",
    "MIGRATION_PROTOCOL_LOG",
    "mkfifo",
    "escape_field",
    "unescape_field",
    "emit_frame",
    "read_frame",
)

_DEFINITION = re.compile(r"^(assert_[a-zA-Z_]+)\s*\(\)\s*\{?\s*$")
_CALL_START = re.compile(r"^(assert_[a-zA-Z_]+)\b")


@dataclass(frozen=True)
class AssertionSite:
    file: str
    line: int
    call: str
    classification: str
    reason: str


def _joined_statement(lines: list[str], start: int) -> tuple[str, int]:
    """Join backslash-continuation lines starting at `start`.

    Returns the joined logical statement and the index of its last
    consumed line.
    """
    statement = lines[start]
    end = start
    while statement.rstrip().endswith("\\") and end + 1 < len(lines):
        end += 1
        statement = statement.rstrip()[:-1] + " " + lines[end].strip()
    return statement, end


def _classify(statement: str, no_cli_surface: bool) -> tuple[str, str]:
    if no_cli_surface:
        return (
            "not-repointable",
            "no CLI surface — drives interactive-protocol.sh directly",
        )
    for marker in NOT_REPOINTABLE_MARKERS:
        if marker in statement:
            return (
                "not-repointable",
                f"references a wire-protocol internal: {marker}",
            )
    return (
        "repointable",
        "asserts CLI-observable stdout/stderr/exit-code/file-state",
    )


def extract_sites(root: Path, relpath: str) -> list[AssertionSite]:
    """Every `assert_*` call site in `relpath`, in file order.

    A commented-out assertion (`# assert_eq ...`) or a call embedded as a
    continuation of a preceding multi-line call is never independently
    matched — only a logical statement's own first physical line anchors a
    site.
    """
    lines = (root / relpath).read_text().splitlines()
    no_cli_surface = relpath in NO_CLI_SURFACE
    sites: list[AssertionSite] = []
    index = 0
    while index < len(lines):
        stripped = lines[index].strip()
        if _DEFINITION.match(stripped):
            index += 1
            continue
        call = _CALL_START.match(stripped)
        if call:
            statement, last = _joined_statement(lines, index)
            classification, reason = _classify(statement, no_cli_surface)
            sites.append(
                AssertionSite(
                    file=relpath,
                    line=index + 1,
                    call=call.group(1),
                    classification=classification,
                    reason=reason,
                )
            )
            index = last + 1
            continue
        index += 1
    return sites


def inventory(
    root: Path, suites: tuple[str, ...] = SUITES
) -> list[AssertionSite]:
    """Every assertion site across `suites`, in the given suite order."""
    sites: list[AssertionSite] = []
    for suite in suites:
        sites.extend(extract_sites(root, suite))
    return sites


def _raw_call_start_count(root: Path, relpath: str) -> int:
    """Count call-starting lines independently of `extract_sites`.

    A single per-line scan with no continuation-consuming or
    index-skipping, unlike `extract_sites`'s traversal. `gaps()` compares
    `inventory()`'s own count against this: they should always agree,
    since a real continuation line (a plain string/variable argument)
    never itself starts with `assert_`. A mismatch means either a
    traversal bug in `extract_sites` or a genuinely tricky line worth a
    manual look.
    """
    count = 0
    for line in (root / relpath).read_text().splitlines():
        stripped = line.strip()
        if _DEFINITION.match(stripped):
            continue
        if _CALL_START.match(stripped):
            count += 1
    return count


def duplicates(sites: list[AssertionSite]) -> list[str]:
    """Every `file:line` claimed by more than one site."""
    seen: set[str] = set()
    dupes: list[str] = []
    for site in sites:
        key = f"{site.file}:{site.line}"
        if key in seen:
            dupes.append(key)
        seen.add(key)
    return dupes


def gaps(
    root: Path, suites: tuple[str, ...] = SUITES
) -> list[tuple[str, int, int]]:
    """Find every suite where the raw count disagrees with the inventory.

    Returns `(suite, raw_count, inventoried_count)` triples.
    """
    mismatches: list[tuple[str, int, int]] = []
    for suite in suites:
        raw = _raw_call_start_count(root, suite)
        found = len(extract_sites(root, suite))
        if raw != found:
            mismatches.append((suite, raw, found))
    return mismatches


def render_table(sites: list[AssertionSite]) -> str:
    """Render a `file:line` / call / classification / reason Markdown table."""
    header = "| Site | Call | Classification | Reason |\n|---|---|---|---|\n"
    rows = "\n".join(
        f"| `{site.file}:{site.line}` | `{site.call}` | "
        f"{site.classification} | {site.reason} |"
        for site in sites
    )
    return header + rows + "\n"


@task
def check(context: Context) -> None:
    """Fail on a duplicate site or a raw/inventoried count mismatch."""
    del context
    root = repo_root()
    sites = inventory(root)
    problems: list[str] = []
    problems.extend(
        f"duplicate assertion site: {dupe}" for dupe in duplicates(sites)
    )
    problems.extend(
        f"{suite}: raw count {raw} != inventoried count {found}"
        for suite, raw, found in gaps(root)
    )
    if problems:
        raise Exit(
            "migrate-suite-inventory found problem(s):\n  "
            + "\n  ".join(problems),
            code=1,
        )


@task
def report(context: Context) -> None:
    """Print the total assertion count and the threshold decision."""
    del context
    sites = inventory(repo_root())
    total = len(sites)
    not_repointable = sum(
        1 for site in sites if site.classification == "not-repointable"
    )
    print(f"Total assertion sites: {total}")
    print(f"  repointable: {total - not_repointable}")
    print(f"  not-repointable: {not_repointable}")
    print(
        "Threshold: exhaustive mapping narrows to the three named suites"
        if total > 400
        else "Threshold: exhaustive mapping applies to all six suites"
    )
